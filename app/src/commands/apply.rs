//! `comtrya apply` and `comtrya status`

mod graph;
mod run;

use super::ComtryaCommand;
use crate::Runtime;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use comtrya_lib::manifests::{load_all, Loaded};
use graph::ManifestGraph;
use run::Run;
use std::ops::Deref;
use std::path::PathBuf;
use tracing::{error, instrument, trace};

/// The arguments of `comtrya apply` and `comtrya status`
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
    /// The directory of the manifests: the first of `manifest_paths` in the config, cloned
    /// first if it's a git repository
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

        let manifest_path = crate::manifests::resolve(first_manifest_path).map_err(|err| {
            anyhow::anyhow!("Can't find the manifests at {first_manifest_path}: {err}")
        })?;

        trace!(manifests = self.manifests.join(",").deref(),);
        Ok(manifest_path)
    }

    /// Lists the manifests with their number of actions, and fails if any can't be loaded
    #[instrument(skip(self, runtime))]
    pub fn status(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let contexts = &runtime.contexts;
        let manifest_path = self.manifest_path(runtime)?;

        println!("Load manifests from path: {:#?}", manifest_path);

        let Loaded { manifests, errors } = load_all(manifest_path, contexts);
        for err in &errors {
            error!("{err}");
        }

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

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "{} manifest(s) can't be loaded",
                errors.len()
            ));
        }

        Ok(())
    }
}

impl ComtryaCommand for Apply {
    /// Loads the manifests, and runs the selected ones with their dependencies first
    #[instrument(skip(self, runtime))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let manifest_path = self.manifest_path(runtime)?;
        let graph = ManifestGraph::build(load_all(manifest_path, &runtime.contexts));

        let mut run = Run::new(
            &graph,
            &runtime.contexts,
            self.dry_run,
            self.label.as_deref(),
        );
        for start in graph.starts(&self.manifests) {
            run.from(start);
        }

        run.finish()
    }
}
