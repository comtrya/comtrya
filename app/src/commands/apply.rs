use super::ComtryaCommand;
use crate::Runtime;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use comtrya_lib::contexts::to_rhai;
use comtrya_lib::manifests::{load, Manifest};
use core::panic;
use petgraph::{visit::DfsPostOrder, Graph};
use rhai::Engine;
use std::path::PathBuf;
use std::{collections::HashMap, ops::Deref};
use tracing::{debug, error, info, instrument, span, trace, warn};

#[derive(Parser, Debug)]
pub(crate) struct Apply {
    /// Run a subset of your manifests, comma separated list.
    /// This should be a list of manifest names. No paths.
    #[arg(short, long, value_delimiter = ',')]
    manifests: Vec<String>,

    /// Performs a dry-run without changing the system
    #[arg(long)]
    dry_run: bool,

    /// Define label selector
    #[arg(short, long)]
    pub label: Option<String>,
}

impl Apply {
    fn manifest_path(&self, runtime: &Runtime) -> anyhow::Result<PathBuf> {
        for manifest in &self.manifests {
            if manifest.contains(std::path::MAIN_SEPARATOR) {
                return Err(anyhow::anyhow!(
                    "Found a path, expected only names in the manifests list!"
                ));
            }
        }

        let first_manifest_path = runtime.config.manifest_paths.first().ok_or_else(|| {
            anyhow::anyhow!(
                "No manifest paths found in config file, please add at least one path to your manifests"
            )
        })?;

        let manifest_path = match crate::manifests::resolve(first_manifest_path) {
            Some(path) => path,
            None => {
                return Err(anyhow::anyhow!(
                    "Manifest location, {:?}, could be resolved",
                    first_manifest_path
                ))
            }
        };

        trace!(manifests = self.manifests.join(",").deref(),);
        Ok(manifest_path)
    }

    #[instrument(skip(self, runtime))]
    pub fn status(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let contexts = &runtime.contexts;
        let manifest_path = self.manifest_path(runtime)?;

        println!("Load manifests from path: {:#?}", manifest_path);

        let manifests = load(manifest_path, contexts);

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(40)
            .set_header(vec!["Manifest", "Count of Actions"]);

        for (name, manifest) in manifests.iter() {
            table.add_row(vec![
                Cell::new(name.to_string()),
                Cell::new(format!("{}", manifest.actions.len())),
            ]);
        }
        println!("{table}");
        Ok(())
    }
}

impl ComtryaCommand for Apply {
    #[instrument(skip(self, runtime))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let contexts = &runtime.contexts;
        let manifest_path = self.manifest_path(runtime)?;
        let manifests = load(manifest_path, contexts);

        // Build DAG
        let mut dag: Graph<Manifest, u32, petgraph::Directed> = Graph::new();

        let manifest_root = Manifest {
            r#where: None,
            root_dir: None,
            dag_index: None,
            name: None,
            depends: vec![],
            actions: vec![],
            ..Default::default()
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
            manifest.depends.iter().for_each(|dependency| {
                let (local_dependency_prefix, _) = name.rsplit_once('.').unwrap_or((name, ""));

                let resolved_dependency_name =
                    dependency.replace("./", format!("{local_dependency_prefix}.").as_str());

                let m1 = match manifests.get(&resolved_dependency_name) {
                    Some(manifest) => manifest,
                    None => {
                        error!(
                            message = "Unresolved dependency",
                            dependency = resolved_dependency_name.as_str()
                        );

                        return;
                    }
                };

                trace!(
                    message = "Dependency Registered",
                    from = name.as_str(),
                    to = m1.name.as_deref().unwrap_or("cannot extract name"),
                );

                if let (Some(from), Some(to)) = (manifest.dag_index, m1.dag_index) {
                    dag.add_edge(from, to, 0);
                } else {
                    error!(message = "Cannot add dependency, missing dag index");
                }
            });
        }

        let clone_m = self.manifests.clone();

        let run_manifests = if self.manifests.is_empty() {
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

        let dry_run = self.dry_run;

        let engine = Engine::new();
        let mut scope = to_rhai(contexts);

        run_manifests.iter().for_each(|manifest| {
            let start = if manifest.eq(&String::from("")) {
                root_index
            } else if let Some(dag_index) = manifests
                .get(manifest)
                .and_then(|manifest| manifest.dag_index)
            {
                dag_index
            } else {
                // FIXME: Don't panic here. Find a better way to handle this.
                panic!("Cannot find manifest in DAG");
            };

            let mut dfs = DfsPostOrder::new(&dag, start);

            while let Some(visited) = dfs.next(&dag) {
                if dag.node_weight(visited).is_none() {
                    info!(
                        message = "Skipping manifest, not found in DAG",
                        index = visited.index()
                    );
                }

                // .unwrap() is safe here, because we just checked that the node exists
                let m1 = dag.node_weight(visited).unwrap();

                // Root manifest, nothing to do.
                if m1.name.is_none() {
                    continue;
                }

                let span_manifest = span!(
                    tracing::Level::INFO,
                    "",
                    manifest = m1.name.as_deref().unwrap_or("Cannot extract name"),
                )
                .entered();

                let mut successful = true;

                if let Some(label) = self.label.as_ref() {
                    if !m1.labels.contains(label) {
                        info!(
                            message = "Skipping manifest, label not found",
                            label = label.as_str()
                        );
                        continue;
                    }
                }

                if let Some(where_condition) = &m1.r#where {
                    let where_result =
                        match engine.eval_with_scope::<bool>(&mut scope, where_condition) {
                            Ok(result) => {
                                debug!(
                                    "Result of 'where' condition '{}' -> '{}'",
                                    where_condition, result
                                );

                                result
                            }
                            Err(err) => {
                                warn!("'where' condition '{}' failed: {}", where_condition, err);
                                false
                            }
                        };

                    if !where_result {
                        info!("Skip manifest, because 'where' conditions were false!");
                        span_manifest.exit();
                        continue;
                    }
                }

                for action in m1.actions.iter() {
                    let span_action = span!(tracing::Level::INFO, "", %action).entered();

                    let action = action.inner_ref();

                    let plan = match action.plan(m1, contexts) {
                        Ok(steps) => steps,
                        Err(err) => {
                            info!("Action failed to get plan: {:?}", err);
                            successful = false;
                            continue;
                        }
                    };

                    let mut steps = plan
                        .into_iter()
                        .filter(|step| step.do_initializers_allow_us_to_run())
                        .filter(|step| match step.atom.plan() {
                            Ok(outcome) => outcome.should_run,
                            Err(_) => false,
                        })
                        .peekable();

                    if steps.peek().is_none() {
                        info!("nothing to be done to reconcile action");
                        span_action.exit();
                        continue;
                    }

                    for mut step in steps {
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
                    info!("{}", action.summarize());
                    span_action.exit();
                }

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
}
