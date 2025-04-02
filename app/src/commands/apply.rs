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

    #[instrument(skip_all)]
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
        let manifest_path = self
            .manifest_path(runtime)
            .inspect(|path| debug!("Load manifests from path: {:#?}", path))?;

        let contexts = runtime.contexts.clone();
        let manifests = load(manifest_path, &contexts);
        let manifest_manager = {
            // Can't have async in closure
            let graph = DependencyGraph::new(manifests, &contexts, runtime).await?;
            Arc::new(LazyLock::new(|| graph))
        };

        let mut workers = JoinSet::<Result<()>>::new();
        let dry_run = self.dry_run;
        let password_manager = &runtime.password_manager;

        for manifest in manifest_manager.get_ordered_manifests() {
            let name = manifest.get_name();

            // Need to clone all these because they'll be in their own threads
            let label = self.label.clone();
            let contexts = contexts.clone();
            let pm = password_manager.clone();
            let manifest_manager = Arc::clone(&manifest_manager);

            workers.spawn(async move {
                // Wait on current manifest's barrier. If a dependency
                // fails it propugates the failure upward as false.
                manifest
                    .barrier
                    .as_ref()
                    .with_context(|| format!("Cannot lock manifest '{}' for execution", name))?
                    .wait(true)
                    .await
                    .then_some(())
                    .context(format!("Skipping manifest '{}' Dependancy(s) failed", name))?;

                let result = manifest.execute(dry_run, label, &contexts, pm).await;

                // Inform successors (dependants) of pass or fail
                for successor in manifest_manager
                    .get_successors(&manifest)
                    .await
                    // should never be None but can't hurt to be careful
                    .context(format!("Cannot resolve dependants for manifest '{}'", name))?
                {
                    successor
                        .barrier
                        .as_ref()
                        .context(format!("Cannot mark manifest '{}' completed", name))?
                        .wait(result.is_ok())
                        .await;
                }

                Ok(())
            });
        }

        while let Some(Err(error)) = workers.join_next().await {
            eprintln!("{error}");
        }

        Ok(())
    }
}
