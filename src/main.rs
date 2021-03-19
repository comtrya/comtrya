mod actions;
mod contexts;
mod manifest;

use crate::actions::{Action, Actions};
use contexts::build_contexts;
use gitsync::GitSync;
use ignore::WalkBuilder;
use manifest::Manifest;
use petgraph::prelude::*;
use std::{collections::HashMap, ops::Deref, time::Duration};
use std::{fs::canonicalize, path::PathBuf};
use structopt::StructOpt;
use tera::Tera;
use tracing::{debug, error, info, span, trace, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(StructOpt, Debug)]
#[structopt(name = "comtrya")]
struct Opt {
    /// Debug & tracing mode (-v, -vv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u8,

    /// Location of manifests (local directory or Git URI)
    #[structopt()]
    manifest_location: String,

    /// Run a subset of your manifests, comma separated list
    #[structopt(short = "m", long, use_delimiter = true)]
    manifests: Vec<String>,
}

fn main() {
    let opt = Opt::from_args();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(true)
        .with_target(false)
        .without_time();

    let subscriber = match opt.verbose {
        0 => subscriber,
        1 => subscriber.with_max_level(Level::DEBUG),
        2 => subscriber.with_max_level(Level::TRACE),
        _ => subscriber.with_max_level(Level::TRACE),
    }
    .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut manifests: HashMap<String, Manifest> = HashMap::new();

    let manifest_directory = match find_manifests(&opt.manifest_location) {
        Some(dir) => dir,
        None => {
            error!("Failed to find manifests at {}", opt.manifest_location);
            panic!();
        }
    };

    trace!(
        manifest_directory = manifest_directory.to_str().unwrap(),
        manifests = opt.manifests.join(",").deref(),
        message = "Comtrya execution started"
    );

    // This should be exposed as a context
    debug!(
        message = "OS Detected",
        OS = os_info::get().os_type().to_string().as_str()
    );

    // Run Context Providers
    let contexts = build_contexts();

    let mut walker = WalkBuilder::new(manifest_directory);
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .build()
        // Don't walk directories
        .filter(|entry| !entry.clone().unwrap().metadata().unwrap().is_dir())
        .filter(|entry| {
            // There has to be a better way to do this.
            // I couldn't get the TypeBuilder to work
            if entry
                .clone()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .ends_with(".yaml")
                || entry
                    .clone()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .ends_with(".yml")
            {
                true
            } else {
                false
            }
        })
        // Don't consider anything in a `files` directory a manifest
        .filter(|entry| {
            !entry
                .clone()
                .unwrap()
                .path()
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .eq("files")
        })
        .for_each(|entry| {
            let filename = entry.unwrap();

            let span = span!(
                tracing::Level::INFO,
                "manifest_load",
                manifest = filename.file_name().to_str().unwrap()
            )
            .entered();

            trace!(manifest = filename.file_name().to_str().unwrap());

            let entry = canonicalize(filename.into_path()).unwrap();

            trace!(absolute_path = entry.to_str().unwrap());

            let contents = std::fs::read_to_string(entry.clone()).unwrap();
            let template = contents.as_str();

            trace!(template = template);

            let yaml = Tera::one_off(template, &contexts, false).unwrap();

            trace!(rendered = yaml.as_str());

            let mut manifest: Manifest = match serde_yaml::from_str(yaml.deref()) {
                Ok(manifest) => manifest,
                Err(e) => {
                    error!(message = e.to_string().as_str());
                    span.exit();

                    return;
                }
            };

            let name = match &manifest.name {
                Some(name) => name.clone(),
                None => {
                    if entry.file_stem().unwrap().to_str().unwrap().eq("main") {
                        // Use directory name for manifest name
                        String::from(
                            entry
                                .parent()
                                .unwrap()
                                .file_stem()
                                .unwrap()
                                .to_str()
                                .unwrap(),
                        )
                    } else {
                        String::from(entry.file_stem().unwrap().to_str().unwrap())
                    }
                }
            };

            manifest.root_dir = Some(entry.parent().unwrap().to_path_buf());

            trace!(message = "Registered Manifest", manifest = name.as_str());

            manifest.name = Some(name.clone());

            manifests.insert(name, manifest);

            span.exit();
        });

    // Build DAG
    let mut dag: Graph<Manifest, u32, petgraph::Directed> = Graph::new();

    let manifest_root = Manifest {
        root_dir: None,
        dag_index: None,
        name: None,
        depends: vec![],
        actions: vec![],
    };

    let root_index = dag.add_node(manifest_root);

    let manifests: HashMap<String, Manifest> = manifests
        .into_iter()
        .map(|(name, mut manifest)| {
            let abc = dag.add_node(manifest.clone());

            manifest.dag_index = Some(abc);
            dag.add_edge(root_index, abc, 0);

            (name, manifest)
        })
        .collect();

    for (name, manifest) in manifests.iter() {
        manifest.depends.iter().for_each(|d| {
            let m1 = match manifests.get(d) {
                Some(manifest) => manifest,
                None => {
                    error!(message = "Unresolved dependency", dependency = d.as_str());

                    return;
                }
            };

            trace!(
                message = "Dependency Registered",
                from = name.as_str(),
                to = m1.name.clone().unwrap().as_str()
            );

            dag.add_edge(manifest.dag_index.unwrap(), m1.dag_index.unwrap(), 0);
        });
    }

    let clone_m = opt.manifests.clone();

    let run_manifests = if (&opt.manifests).is_empty() {
        // No manifests specified on command line, so run everything
        vec![String::from("")]
    } else {
        // Run subset
        manifests
            .keys()
            .filter(|z| clone_m.contains(z))
            .cloned()
            .collect::<Vec<String>>()
    };

    debug!(manifests = run_manifests.join(",").as_str());

    run_manifests.iter().for_each(|m| {
        let start = if m.eq(&String::from("")) {
            root_index
        } else {
            manifests.get(m).unwrap().dag_index.unwrap()
        };

        let mut dfs = DfsPostOrder::new(&dag, start);

        while let Some(visited) = dfs.next(&dag) {
            let m1 = dag.node_weight(visited).unwrap();

            // Root manifest, nothing to do.
            if m1.name.is_none() {
                continue;
            }

            let span_manifest = span!(
                tracing::Level::INFO,
                "manifest_run",
                manifest = m1.name.clone().unwrap().as_str()
            )
            .entered();

            let mut successful = true;

            m1.actions.iter().for_each(|action| {
                let result = match action {
                    Actions::DirectoryCopy(a) => a.run(m1, &contexts),
                    Actions::FileCopy(a) => a.run(m1, &contexts),
                    Actions::FileLink(a) => a.run(m1, &contexts),
                    Actions::PackageInstall(a) => a.run(&m1, &contexts),
                };

                match result {
                    Ok(result) => {
                        debug!("{}", result.message)
                    }
                    Err(e) => {
                        successful = false;

                        error!(message = e.message.as_str())
                    }
                }
            });

            if successful {
                info!("Completed");
            } else {
                error!("Failed");
                span_manifest.exit();
                break;
            }

            span_manifest.exit();
        }
    });
}

fn find_manifests(location: &String) -> Option<PathBuf> {
    if location.starts_with("/") {
        return match PathBuf::from(location).canonicalize() {
            Ok(location) => Some(location),
            Err(_) => {
                error!("Failed to locate manifests at {}", location);

                None
            }
        };
    }

    // Assume Git
    if location.starts_with("http") {
        // Extract this to a function!
        let clean_repo_url = String::from(location.as_str())
            .replace("https", "")
            .replace("http", "")
            .replace(":", "")
            .replace(".", "")
            .replace("/", "");

        let cache_path = dirs_next::cache_dir().unwrap().join(clean_repo_url);

        let git_sync = GitSync {
            repo: location.clone(),
            dir: cache_path.clone(),
            branch: String::from("main"),
            passphrase: None,
            private_key: None,
            sync_every: Duration::from_secs(5),
            username: None,
        };

        println!("Syncing repo to {}", cache_path.clone().to_str().unwrap());

        if let Err(error) = git_sync.bootstrap() {
            error!("Failed to bootstrap repository, {:?}", error);
            return None;
        }

        if let Err(error) = git_sync.sync() {
            println!("{:?}: FUCK", error);
            error!("Failed to bootstrap repository, {:?}", error);
            return None;
        }

        return Some(cache_path.clone());
    }

    // Try relative path
    return match std::env::current_dir()
        .unwrap()
        .join(location)
        .canonicalize()
    {
        Ok(location) => Some(location),
        Err(_) => {
            error!("Failed to locate manifests at {}", location);
            None
        }
    };
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::find_manifests;

    #[test]
    fn it_can_handle_non_existant_directories() {
        let fake = String::from("/fake");

        assert_eq!(true, find_manifests(&fake).is_none());
    }

    #[test]
    fn it_can_handle_relative_directories() {
        // Setup a temp directory and set cwd
        let temp_cwd = TempDir::new("relative-dirs").unwrap();
        std::env::set_current_dir(temp_cwd.path()).unwrap();

        // Create a directory within cwd, so canonicalize won't fail
        let location = String::from("./hello");
        std::fs::create_dir(temp_cwd.path().join(&location)).unwrap();

        // Pass in a relative path, get an absolute path back
        assert_eq!(
            temp_cwd.path().join("hello").canonicalize().unwrap(),
            find_manifests(&location).unwrap()
        );
    }

    #[test]
    fn it_can_handle_absolute_directories() {
        let temp_cwd = TempDir::new("relative-dirs").unwrap();
        let location = String::from(temp_cwd.path().canonicalize().unwrap().to_str().unwrap());

        // Pass in an absolute path, get the same back
        assert_eq!(
            temp_cwd.path().canonicalize().unwrap(),
            find_manifests(&location).unwrap()
        );
    }

    #[test]
    fn it_can_handle_git_uris() {
        let location = String::from("https://github.com/comtrya/comtrya");

        let git_cache_git = dirs_next::cache_dir()
            .unwrap()
            .join("githubcomcomtryacomtrya");

        assert_eq!(git_cache_git, find_manifests(&location).unwrap());
    }
}
