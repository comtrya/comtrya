use crate::actions::{Action, Actions};
use ignore::{types::TypesBuilder, WalkBuilder};
use petgraph::prelude::*;
use regex::Regex;
use std::{collections::HashMap, io::Result, ops::Deref};
use std::{fs::canonicalize, path::PathBuf};
use structopt::StructOpt;
use tera::Tera;

mod actions;
mod contexts;
use contexts::build_contexts;
mod manifest;
use manifest::Manifest;

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

    // Run Context Providers
    let contexts = build_contexts();

    // Find Manifests
    let mut yaml_filter = TypesBuilder::new();
    yaml_filter.add("yaml", "*.yaml").unwrap();
    yaml_filter.add("yml", "*.yml").unwrap();

    let mut walker = WalkBuilder::new(PathBuf::from(opt.manifest_directory));
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .types(yaml_filter.build().unwrap())
        .build()
        .filter(|entry| !entry.clone().unwrap().metadata().unwrap().is_dir())
        .for_each(|entry| {
            let filename = entry.unwrap();

            println!("found {:?}", filename);

            let entry = canonicalize(filename.into_path()).unwrap();

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
            mo.root_dir = Some(entry.clone().parent().unwrap().to_path_buf());

            println!("Registering Manifest {:?}", name);

            mo.name = Some(name.clone());

            manifests.insert(name, mo);

            ()
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
            println!("Actions: {:?}", m1.actions);

            m1.actions.iter().for_each(|action| {
                let result = match action {
                    Actions::PackageInstall(a) => a.run(&m1, &contexts),
                    Actions::FileCopy(a) => a.run(m1, &contexts),
                };

                match result {
                    Ok(o) => println!("OK: {:?}", o),
                    Err(e) => println!("Err: {:?}", e),
                }

                ()
            });
        }
    });

    Ok(())
}
