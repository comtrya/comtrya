use petgraph::prelude::*;
use serde_json::value::Value;
use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    io::Result,
    ops::Deref,
};
use std::{ffi::OsString, path::PathBuf};
use structopt::StructOpt;
use tera::{Context, Tera};
use walkdir::WalkDir;

mod actions;

mod contexts;
use contexts::user::UserContextProvider;
use contexts::ContextProvider;

mod files;

mod packages;
use packages::Package;

mod manifests;
use manifests::Manifest;

#[derive(StructOpt, Debug)]
#[structopt(name = "comtrya")]
struct Opt {
    /// Activate debug mode
    #[structopt(long)]
    debug: bool,

    /// Directory where manifests are located
    #[structopt(short = "d", long, parse(from_os_str), default_value = ".")]
    manifest_directory: PathBuf,

    /// Run a subset of your manifests, comma separated list
    #[structopt(short = "m", long, use_delimiter = true)]
    manifests: Vec<String>,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let mut manifests: HashMap<String, Manifest> = HashMap::new();
    let root_dir = std::env::current_dir()
        .unwrap()
        .join(opt.manifest_directory.clone());

    let mut tera = match Tera::new(format!("{}/**/*", root_dir.clone().to_str().unwrap()).deref()) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    // Load all supported PackageProviders
    // let mut package_providers: HashMap<PackageProviders, > = HashMap::new();

    // Run Context Providers
    let mut contexts = Context::new();
    let context_providers: Vec<Box<dyn ContextProvider>> = vec![Box::new(UserContextProvider {})];

    context_providers.iter().for_each(|provider| {
        let mut values: BTreeMap<String, Value> = BTreeMap::new();

        provider.get_contexts().iter().for_each(|context| {
            match context {
                contexts::Context::KeyValueContext(k, v) => {
                    values.insert(k.clone(), v.clone().into());
                    ()
                }
                contexts::Context::ListContext(k, v) => {
                    values.insert(k.clone(), v.clone().into());
                    ()
                }
            }

            ()
        });

        contexts.insert(provider.get_prefix(), &values);

        ()
    });

    println!("Contexts for this execution: {:?}", contexts);

    // Find Manifests
    for entry in WalkDir::new(opt.manifest_directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|d| d.into_path())
        .filter(|p| p.extension().is_some())
        // If the parent directory is files, we assume it's a template.
        // I'm not sure how I feel about this yet
        .filter(|p| match p.parent() {
            Some(p) => p
                .file_stem()
                .unwrap_or(&OsStr::new("not_files"))
                .ne("files"),
            None => false,
        })
        .filter(|p| ["yaml", "yml"].contains(&(p.extension().unwrap().to_str().unwrap())))
    {
        let contents = std::fs::read_to_string(entry.clone()).unwrap();
        let template = contents.as_str();

        println!("Template is {:?}", template);

        let yaml = tera.render_str(template, &contexts).unwrap();

        let mut mo: Manifest = serde_yaml::from_str(yaml.deref()).unwrap();

        let name = match &mo.name {
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

        println!(
            "Root Dir for Manifest: {}",
            entry
                .clone()
                .parent()
                .unwrap()
                .strip_prefix(root_dir.clone())
                .unwrap()
                .to_str()
                .unwrap()
        );
        mo.root_dir = Some(
            entry
                .clone()
                .parent()
                .unwrap()
                .strip_prefix(root_dir.clone())
                .unwrap()
                .to_path_buf(),
        );

        println!("Registering Manifest {:?}", name);

        mo.name = Some(name.clone());

        manifests.insert(name, mo);

        ()
    }

    // Build DAG
    let mut dag: Graph<Manifest, u32, petgraph::Directed> = Graph::new();

    let manifest_root = Manifest {
        root_dir: None,
        dag_index: None,
        name: None,
        depends: vec![],
        packages: vec![],
        files: vec![],
    };
    let root_index = dag.add_node(manifest_root);

    let manifests: HashMap<String, Manifest> = manifests
        .into_iter()
        .map(|(name, mut manifest)| {
            let abc = dag.add_node(manifest.clone());

            manifest.dag_index = Some(abc);
            dag.add_edge(root_index, abc, 0);

            (name.clone(), manifest)
        })
        .collect();

    for (name, manifest) in manifests.iter() {
        manifest.depends.iter().for_each(|d| {
            let m1 = manifests.get(d).unwrap();

            println!("Found that {:?} has a dep on {:?}", name, m1.name);

            dag.add_edge(manifest.dag_index.unwrap(), m1.dag_index.unwrap(), 0);
        });
    }

    println!("opt manifests is {:?}", opt.manifests);
    let clone_m = opt.manifests.clone();

    let run_manifests = if (&opt.manifests).is_empty() {
        // No manifests specified on command line, so run everything
        vec![String::from("")]
    } else {
        // Run subset
        manifests
            .keys()
            .filter(|z| clone_m.contains(z.clone()))
            .map(|z| z.clone())
            .collect::<Vec<String>>()
    };

    println!("run manifests {:?}", run_manifests);

    run_manifests.iter().for_each(|m| {
        let start = if m.eq(&String::from("")) {
            root_index
        } else {
            manifests.get(m).unwrap().dag_index.unwrap()
        };

        let mut dfs = DfsPostOrder::new(&dag, start);

        while let Some(visited) = dfs.next(&dag) {
            let m1 = dag.node_weight(visited).unwrap();

            println!("Walking {:?}", m1.name);

            // Root manifest, nothing to do.
            if m1.name.is_none() {
                continue;
            }

            println!("Provisioning Manifest: {:?}", m1.name.clone().unwrap());

            for p in m1.packages.clone().into_iter() {
                let mut p = Package::from(p);
                let provider = packages::get_provider(p.provider.clone());

                if provider.is_none() {
                    println!("Couldn't find a provider to install {:?}", p.list.join(" "));
                    continue;
                }

                let provider = provider.unwrap();

                match provider.init() {
                    Ok(true) => {
                        println!("Installed package provider");
                        ()
                    }

                    Ok(false) => (),

                    Err(_) => {
                        println!("Failed to install package provider");
                        ()
                    }
                }

                if p.list.is_empty() {
                    p.list = vec![m1.name.clone().unwrap()];
                }

                println!("INSTALL");
                provider.install(&p);

                continue;
            }

            for f in m1.clone().files.into_iter() {
                println!("Manifest {:?} - Files working {:?}", m1.name, f);

                let abc = match f.symlink.unwrap_or(false) {
                    true => m1.link(f, &tera),
                    false => m1.create(f, &tera, &contexts),
                };

                match abc {
                    Ok(_) => println!("File creation was ok"),
                    Err(_) => println!("File creation failed"),
                }
            }
        }
    });

    Ok(())
}
