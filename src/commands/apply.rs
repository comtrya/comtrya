use crate::{contexts::build_contexts, manifests::load};
use petgraph::{visit::DfsPostOrder, Graph};
use std::{collections::HashMap, ops::Deref};
use structopt::StructOpt;
use tracing::{debug, error, info, span, trace};

use crate::{config::Config, manifests::Manifest};

#[derive(Clone, Debug, StructOpt)]
pub(crate) struct Apply {
    /// Run a subset of your manifests, comma separated list
    #[structopt(short = "m", long, use_delimiter = true)]
    manifests: Vec<String>,

    /// Performs a dry-run without changing the system
    #[structopt(long)]
    dry_run: bool,
}

pub(crate) fn execute(args: Apply, config: Config) -> anyhow::Result<()> {
    let manifest_path = match crate::manifests::resolve(config.manifest_paths.first().unwrap()) {
        Some(path) => path,
        None => {
            return Err(anyhow::anyhow!(
                "Manifest location, {:?}, could be resolved",
                config.manifest_paths.first().unwrap()
            ))
        }
    };

    trace!(
        manifest_path = manifest_path.to_str().unwrap(),
        manifests = args.manifests.join(",").deref(),
        message = "Comtrya execution started"
    );

    // This should be exposed as a context
    debug!(
        message = "OS Detected",
        OS = os_info::get().os_type().to_string().as_str()
    );

    let contexts = build_contexts(&config);

    let manifests = load(manifest_path, &contexts);

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

    let clone_m = args.manifests.clone();

    let run_manifests = if (&args.manifests).is_empty() {
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

    let dry_run = args.dry_run;
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
                    .plan(m1, &contexts)
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
