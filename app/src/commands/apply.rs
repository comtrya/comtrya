use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use petgraph::{graph::NodeIndex, Direction::*};
use tokio::{
    sync::{Barrier, RwLock},
    task::JoinSet,
};
use tracing::{debug, info, instrument, trace};

use super::ComtryaCommand;
use crate::{utils::DependencyGraph, Runtime};
use comtrya_lib::manifests::load;

#[derive(Parser, Debug, Clone)]
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
                return Err(anyhow!(
                    "Found a path, expected only names in the manifests list!"
                ));
            }
        }

        let first_manifest_path = runtime.config.manifest_paths.first().context(
            "No manifest paths found in config file, please add at least one path to your manifests"
        )?;

        let manifest_path = crate::manifests::resolve(first_manifest_path).context(format!(
            "Manifest location, {first_manifest_path:?}, could be resolved",
        ))?;

        trace!(manifests = self.manifests.join(","));

        Ok(manifest_path)
    }

    #[instrument(skip(self, runtime))]
    pub async fn status(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let contexts = &runtime.contexts;
        let manifest_path = self.manifest_path(runtime)?;

        println!("Load manifests from path: {manifest_path:#?}",);

        let manifests = load(manifest_path, contexts);

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(40)
            .set_header(vec!["Manifest", "Count of Actions"]);

        for (name, manifest) in manifests.iter() {
            table.add_row(vec![
                Cell::new(name),
                Cell::new(format!("{}", manifest.actions.len())),
            ]);
        }

        println!("{table}");

        Ok(())
    }
}

impl ComtryaCommand for Apply {
    // #[instrument(skip(self, runtime))]
    async fn execute(&self, runtime: &mut Runtime) -> Result<()> {
        let contexts = runtime.contexts.clone();
        let manifest_path = self.manifest_path(runtime)?;

        debug!("Load manifests from path: {manifest_path:#?}");

        let manifests = load(manifest_path, &contexts);

        let manifest_manager = Arc::new(RwLock::new(
            DependencyGraph::new(manifests, &contexts, runtime).await?,
        ));

        let mut barrier_map: HashMap<NodeIndex<u32>, Arc<Barrier>> = HashMap::new();
        for node in manifest_manager.read().await.graph.node_indices() {
            barrier_map.insert(
                node,
                Arc::new(Barrier::new(
                    manifest_manager
                        .read()
                        .await
                        .graph
                        .neighbors_directed(node, Incoming)
                        .count()
                        .max(1),
                )),
            );
        }

        let ordered_manifests = manifest_manager.read().await.get_ordered_manifests();
        let mut workers = JoinSet::<Result<()>>::new();
        let dry_run = Arc::new(AtomicBool::new(self.dry_run));
        let password_manager = &runtime.password_manager;
        let barrier_map = Arc::new(barrier_map);
        for manifest in ordered_manifests {
            // Need to clone all these because they'll be in their own threads
            let dry_run = Arc::clone(&dry_run);
            let label = self.label.clone();
            let contexts = contexts.clone();
            let password_manager = password_manager.clone();
            let barrier_map = Arc::clone(&barrier_map);
            let manager = Arc::clone(&manifest_manager);
            let barrier = Arc::clone(
                barrier_map
                    .get(manager.read().await.get_node_from_manifest(&manifest).await)
                    .unwrap(),
            );

            workers.spawn(async move {
                let manifest = Arc::clone(&manifest);
                let manifest_guard = manifest.lock().await;
                let manifest_manager = Arc::clone(&manager);

                let dependents: Vec<_> = manifest_manager
                    .read()
                    .await
                    .graph
                    .neighbors_directed(
                        *manifest_manager
                            .read()
                            .await
                            .get_node_from_manifest(&manifest)
                            .await,
                        petgraph::Outgoing,
                    )
                    .collect();

                for dep in dependents.iter() {
                    debug!(
                        "Dependencies for manifest {}: {:?}",
                        manifest_guard,
                        manifest_manager
                            .read()
                            .await
                            .graph
                            .node_weight(*dep)
                            .unwrap()
                            .lock()
                            .await
                            .get_name()
                    );
                }

                info!(
                    "Worker for manifest {:?} waiting on dependencies",
                    &manifest_guard.get_name()
                );

                // let is_leader = barrier.wait().await.is_leader();
                for &dependent in &dependents {
                    let dependent_barrier = barrier_map.get(&dependent).unwrap();
                    dependent_barrier.wait().await;
                }

                info!(
                    "Worker for manifest {:?} executing manifest",
                    &manifest_guard.get_name()
                );

                manifest_guard
                    .execute(
                        dry_run.load(Ordering::Relaxed),
                        label.clone(),
                        &contexts,
                        password_manager.clone(),
                    )
                    .await?;

                info!(
                    "Worker for manifest {:?} finished executing",
                    &manifest_guard.get_name()
                );

                let has_dependees = manifest_manager
                    .read()
                    .await
                    .graph
                    .neighbors_directed(
                        *manifest_manager
                            .read()
                            .await
                            .get_node_from_manifest(&manifest)
                            .await,
                        Outgoing,
                    )
                    .peekable()
                    .peek()
                    .is_none();

                if has_dependees {
                    barrier.wait().await;
                }
                info!(
                    "Worker for manifest {:?} FINISHED executing",
                    &manifest_guard.get_name()
                );
                Ok(())
            });
        }

        workers.join_all().await;
        Ok(())
    }
}
