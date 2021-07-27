use anyhow::anyhow;
use ignore::WalkBuilder;
use manifests::Manifest;
use petgraph::prelude::*;
use std::fs::canonicalize;
use std::{collections::HashMap, ops::Deref};
use structopt::StructOpt;
use tera::Tera;
use tracing::{debug, error, info, span, trace, Level, Subscriber};
use tracing_subscriber::FmtSubscriber;

use crate::config::load_config;
use crate::contexts::{build_contexts, to_tera};

mod actions;
mod atoms;
mod config;
mod contexts;
mod manifests;
mod steps;

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

#[derive(StructOpt, Clone, Debug)]
#[structopt(name = "comtrya")]
pub struct Opt {
    /// Performs a dry-run without changing the system
    #[structopt(long)]
    dry_run: bool,

    /// Debug & tracing mode (-v, -vv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u8,

    /// Location of manifests (local directory or Git URI)
    #[structopt()]
    manifest_location: Option<String>,

    /// Run a subset of your manifests, comma separated list
    #[structopt(short = "m", long, use_delimiter = true)]
    manifests: Vec<String>,

    /// Disable color printing
    #[structopt(long = "no-color")]
    no_color: bool,

    /// Print the version
    #[structopt(long = "version", short = "V")]
    print_version: bool,
}

fn configure_subscriber(opt: &Opt) -> impl Subscriber {
    let builder = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(!opt.no_color)
        .with_target(false)
        .without_time();

    match opt.verbose {
        0 => builder,
        1 => builder.with_max_level(Level::DEBUG),
        2 => builder.with_max_level(Level::TRACE),
        _ => builder.with_max_level(Level::TRACE),
    }
    .finish()
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    if opt.print_version {
        println!("{}", VERSION.unwrap_or("unknown"));
        return Ok(());
    }

    let subscriber = configure_subscriber(&opt);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = match load_config(opt.clone()) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", error.to_string());
            panic!();
        }
    };

    let manifest_location = match config.manifests.first() {
        Some(l) => l,
        None => {
            return Err(anyhow!("No manifest location provided"));
        }
    };

    let mut manifests: HashMap<String, Manifest> = HashMap::new();

    let manifest_directory = manifests::register_providers()
        .into_iter()
        .filter(|provider| provider.deref().looks_familiar(&manifest_location))
        .fold(None, |path, provider| {
            if path.is_some() {
                return path;
            }

            match provider.resolve(&manifest_location) {
                Ok(path) => Some(path),
                Err(_) => None,
            }
        });

    let manifest_directory = match manifest_directory {
        Some(dir) => dir,
        None => {
            error!("Failed to find manifests at {}", &manifest_location);
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
            entry
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

            let yaml = Tera::one_off(template, &to_tera(&contexts), false).unwrap();

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

    let dry_run = opt.dry_run;
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
                let action = action.inner_ref();

                let mut steps = action
                    .plan(&m1, &contexts)
                    .into_iter()
                    .filter(|step| step.do_initializers_allow_us_to_run())
                    .filter(|step| step.atom.plan())
                    .peekable();

                if steps.peek().is_none() {
                    info!("Nothing to be done to reconcile manifest");
                    return;
                }

                for mut step in steps {
                    info!("{}", step);

                    if dry_run {
                        continue;
                    }

                    match step.atom.execute() {
                        Ok(_) => (),
                        Err(err) => {
                            debug!("Atom failed to execute: {:?}", err);
                            successful = false;
                            break;
                        }
                    }

                    if !step.do_finalizers_allow_us_to_continue() {
                        debug!("Finalizers won't allow us to continue with this action");
                        successful = false;
                        break;
                    }
                }
            });

            if dry_run {
                span_manifest.exit();
                continue;
            }

            if !successful {
                error!("Failed");
                span_manifest.exit();
                break;
            }

            info!("Completed");
            span_manifest.exit();
        }
    });

    Ok(())
}
