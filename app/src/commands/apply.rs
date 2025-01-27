use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use tokio::task::JoinSet;
use tracing::{debug, instrument, trace};

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
    #[instrument(skip_all)]
    async fn execute(&self, runtime: &mut Runtime) -> Result<()> {
        let contexts = runtime.contexts.clone();
        let manifest_path = self.manifest_path(runtime)?;
        debug!("Load manifests from path: {manifest_path:#?}");
        let manifests = load(manifest_path, &contexts);

        let manifest_manager = {
            let graph = DependencyGraph::new(manifests, &contexts, runtime).await?;
            Arc::new(LazyLock::new(|| graph))
        };

        let mut workers = JoinSet::<Result<()>>::new();
        let dry_run = self.dry_run;
        let password_manager = &runtime.password_manager;

        for manifest in manifest_manager.get_ordered_manifests() {
            // Need to clone all these because they'll be in their own threads
            let label = self.label.clone();
            let contexts = contexts.clone();
            let password_manager = password_manager.clone();
            let manifest_manager = Arc::clone(&manifest_manager);

            workers.spawn(async move {
                for successor in manifest_manager.get_successors(&manifest).await {
                    manifest_manager.get_barrier(successor).wait().await;
                }

                manifest
                    .execute(dry_run, label.clone(), &contexts, password_manager.clone())
                    .await?;

                manifest_manager
                    .get_barrier(*manifest_manager.get_node_from_manifest(&manifest).await)
                    .wait()
                    .await;
                Ok(())
            });
        }

        workers.join_all().await;
        Ok(())
    }
}
