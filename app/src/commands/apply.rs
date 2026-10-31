//! `comtrya apply` and `comtrya status`

/// Showing the events of a run
mod log;
/// A status line for steps that take a while
mod progress;

use super::ComtryaCommand;
use crate::Runtime;
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use comtrya_lib::{
    actions::PlanCx,
    executor::{self, Options},
    manifests::{load_all, Loaded},
};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, instrument, trace, warn};

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

    /// Run up to this many manifests at once, each when the manifests it depends on are done.
    /// Steps that change the same files or package database take turns
    #[arg(short, long, default_value_t = 1, value_parser = clap::value_parser!(u16).range(1..))]
    jobs: u16,
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

    /// The manifests to run: those of `-m`, else `default_manifests` from the config. Empty
    /// means all
    fn selected<'a>(&'a self, runtime: &'a Runtime) -> &'a [String] {
        if self.manifests.is_empty() {
            &runtime.config.default_manifests
        } else {
            &self.manifests
        }
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

    /// The status line for long steps, when the log goes to a terminal, see
    /// [`progress::wanted`]
    fn progress(&self, runtime: &Runtime) -> Option<progress::Progress> {
        use std::io::IsTerminal;

        let terminal = std::io::stdout().is_terminal();
        progress::wanted(terminal, runtime.args.verbose, self.dry_run, self.jobs)
            .then(progress::Progress::stdout)
    }
}

/// Set by the first Ctrl-C: the step that's running finishes, nothing starts after it. A
/// second Ctrl-C quits at once
fn stop_on_ctrl_c() -> &'static AtomicBool {
    static STOP: AtomicBool = AtomicBool::new(false);

    let handled = ctrlc::set_handler(|| {
        if STOP.swap(true, Ordering::Relaxed) {
            std::process::exit(130);
        }
        warn!("Stopping after the current step, press Ctrl-C again to quit now");
    });
    if let Err(err) = handled {
        warn!("Ctrl-C quits at once: {err}");
    }

    &STOP
}

impl ComtryaCommand for Apply {
    /// Loads the manifests, and runs the selected ones with their dependencies first
    #[instrument(skip(self, runtime))]
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let manifest_path = self.manifest_path(runtime)?;

        executor::run(
            load_all(manifest_path.clone(), &runtime.contexts),
            &manifest_path,
            PlanCx::new(&runtime.facts, &runtime.contexts),
            &Options {
                manifests: self.selected(runtime),
                dry_run: self.dry_run,
                label: self.label.as_deref(),
                observer: &log::Log::new(self.dry_run, self.progress(runtime)),
                cancel: Some(stop_on_ctrl_c()),
                jobs: self.jobs.into(),
            },
        )
        .into_result()
    }
}
