//! Applies manifests: dependencies before the manifests that need them, each manifest's actions
//! in order, and each action's steps that are needed.
//!
//! [`run()`] takes the loaded manifests and walks them as a graph of their `depends`. For each
//! manifest it evaluates `where` and the `--label`, then plans each action and runs its steps
//! one by one. A step runs only when its atom's check finds drift; a dry run stops after the
//! check. A failing manifest stops itself and the manifests that depend on it, not the others,
//! and ends up in the [`Report`].
//!
//! With `jobs` in [`Options`] above 1, manifests whose dependencies are done run at the same time.
//! Steps that use the same [`Resource`](crate::atoms::Resource), like a file or the package
//! database, take turns.
//!
//! The executor doesn't write the output itself. It tells an [`Observer`] each [`Event`]: a
//! manifest starts, a step is skipped or runs, a line of a program's output, a failure. The
//! command line's observer turns them into the log and the progress line. This one keeps the
//! steps that run, and shows that applying twice changes nothing the second time:
//!
//! ```
//! use comtrya_lib::actions::PlanCx;
//! use comtrya_lib::contexts::Facts;
//! use comtrya_lib::executor::{self, Event, Observer, Options};
//! use comtrya_lib::manifests::load_all;
//! use std::sync::Mutex;
//!
//! /// Keeps every step that runs
//! #[derive(Default)]
//! struct Steps(Mutex<Vec<String>>);
//!
//! impl Observer for Steps {
//!     fn on(&self, event: &Event) {
//!         if let Event::StepStarted(step) = event {
//!             self.0.lock().unwrap().push(step.to_string());
//!         }
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let manifests = tempfile::tempdir()?;
//! let new = manifests.path().join("new");
//! let manifest = format!("actions:\n  - action: dir.create\n    path: '{}'\n", new.display());
//! std::fs::write(manifests.path().join("demo.yaml"), manifest)?;
//!
//! let facts = Facts::default();
//! let contexts = facts.to_contexts();
//! let apply = || {
//!     let steps = Steps::default();
//!     let options = Options { observer: &steps, ..Options::default() };
//!     let loaded = load_all(manifests.path().to_path_buf(), &contexts);
//!     executor::run(loaded, manifests.path(), PlanCx::new(&facts, &contexts), &options);
//!     steps.0.into_inner().unwrap()
//! };
//!
//! assert_eq!(apply(), vec![format!("Create directory {}", new.display())]);
//! assert!(apply().is_empty()); // the directory is there now, nothing to do
//! # Ok(())
//! # }
//! ```

/// What the executor tells about a run
mod event;
/// The manifests and their dependencies as a graph
mod graph;
/// Keeping steps that run at once from using the same things
mod locks;
/// Walking the graph and running the steps
mod run;
/// Which manifest runs next, and how many at once
mod schedule;

use crate::{actions::PlanCx, manifests::Loaded};
pub use event::{Event, Ignore, Observer, Skip, StepSkip};
use graph::ManifestGraph;
use run::Run;
use std::{path::Path, sync::atomic::AtomicBool};
use thiserror::Error as ThisError;

/// What to run, and how
#[derive(Clone, Copy)]
pub struct Options<'a> {
    /// Only these manifests, with their dependencies. All of them if empty
    pub manifests: &'a [String],
    /// Only plan and show what would change
    pub dry_run: bool,
    /// Only manifests with this label
    pub label: Option<&'a str>,
    /// Receives what happens, see [`Event`]
    pub observer: &'a dyn Observer,
    /// When this becomes true, e.g. on Ctrl-C, the step that's running finishes and nothing
    /// starts after it
    pub cancel: Option<&'a AtomicBool>,
    /// How many manifests may run at once, each in its own thread, when the manifests they
    /// depend on are done. Steps that use the same things take turns, see
    /// [`crate::atoms::Resource`]. 1 runs them one after the other, a dry run always does
    pub jobs: usize,
}

impl Options<'_> {
    /// How many manifests run at once: a dry run notes what earlier steps would make, so it
    /// keeps the order
    fn jobs(&self) -> usize {
        match self.dry_run {
            true => 1,
            false => self.jobs.max(1),
        }
    }
}

impl Default for Options<'_> {
    /// All manifests, for real, and nobody hears about it
    fn default() -> Self {
        Self {
            manifests: &[],
            dry_run: false,
            label: None,
            observer: &Ignore,
            cancel: None,
            jobs: 1,
        }
    }
}

/// What a run did
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The manifests that failed, couldn't be loaded, or were skipped because a dependency
    /// failed, by name
    pub failed: Vec<String>,
    /// Whether the run stopped early, see [`Options::cancel`]
    pub cancelled: bool,
}

/// Why a run didn't succeed
#[derive(Debug, ThisError, PartialEq, Eq)]
pub enum RunError {
    /// The run stopped early, see [`Options::cancel`]
    #[error("Cancelled")]
    Cancelled,
    /// Manifests failed
    #[error("{0} manifest(s) failed or were skipped because a dependency failed")]
    Failed(usize),
}

impl Report {
    /// An error if any manifest failed, or the run was cancelled
    ///
    /// # Errors
    ///
    /// [`RunError::Cancelled`] after Ctrl-C, else [`RunError::Failed`] with the number of
    /// manifests that failed or were skipped
    pub fn into_result(self) -> Result<(), RunError> {
        if self.cancelled {
            return Err(RunError::Cancelled);
        }
        if self.failed.is_empty() {
            return Ok(());
        }

        Err(RunError::Failed(self.failed.len()))
    }
}

/// Runs the manifests loaded from `manifest_dir`. A manifest that fails stops only itself and
/// the manifests that depend on it
pub fn run(loaded: Loaded, manifest_dir: &Path, cx: PlanCx, options: &Options) -> Report {
    let graph = ManifestGraph::build(loaded, manifest_dir);

    let run = Run::new(&graph, cx, options);
    run.all(graph.run_order(options.manifests));
    run.finish()
}

#[cfg(test)]
mod tests;
