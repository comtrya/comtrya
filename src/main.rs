use petgraph::prelude::*;
use serde_json::value::Value;
use std::path::PathBuf;
use std::{
    collections::{BTreeMap, HashMap},
    io::Result,
    ops::Deref,
};
use structopt::StructOpt;
use tera::{Context, Tera};
use walkdir::WalkDir;

mod contexts;
use contexts::user::UserContextProvider;
use contexts::ContextProvider;

mod files;

mod packages;
use packages::{PackageConfig, ProviderPackage};

mod manifests;
use manifests::Manifest;

#[derive(StructOpt, Debug)]
#[structopt(name = "comtrya")]
struct Opt {
    /// Activate debug mode
    #[structopt(short, long)]
    debug: bool,

    /// Directory where manifests are located
    #[structopt(short, long, parse(from_os_str))]
    manifest_directory: PathBuf,
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

    // Walk DAG / Run Manifests
    let mut dfs = DfsPostOrder::new(&dag, root_index);

    while let Some(visited) = dfs.next(&dag) {
        let m1 = dag.node_weight(visited).unwrap();

        // Root manifest, nothing to do.
        if m1.name.is_none() {
            continue;
        }

        println!("Provisioning Manifest: {:?}", m1.name.clone().unwrap());

        for p in m1.packages.clone().into_iter() {
            let p = ProviderPackage::from(p);

            // let result = p.run_command();
            // match result.0 {
            //     Ok(_) => println!("Manifest {:?} - Packages Suceeded", p.name()),
            //     Err(_) => {
            //         println!("Manifest {:?} Failed", p.name());

            //         continue;
            //     }
            // }
            continue;
        }

        for f in m1.clone().files.into_iter() {
            println!("Manifest {:?} - Files working {:?}", m1.name, f);

            let abc = match f.symlink.unwrap_or(false) {
                true => m1.link(f),
                false => m1.create(f, &tera, &contexts),
            };

            match abc {
                Ok(_) => println!("File creation was ok"),
                Err(_) => println!("File creation failed"),
            }
        }
    }

    Ok(())
}
