//! What the executor tells about a run, for the command line to show

use crate::{actions, atoms, atoms::command::Stream, steps::Step};
use std::path::Path;

/// Something that happened in a run. The events of a manifest come between its
/// [`Event::ManifestStarted`] and its [`Event::ManifestSkipped`] or [`Event::ManifestDone`], and
/// the events of an action between its [`Event::ActionStarted`] and its
/// [`Event::ActionPlanFailed`] or [`Event::ActionDone`]
pub enum Event<'a> {
    /// A manifest of the run can't run at all: it can't be loaded, or it depends on a manifest
    /// that doesn't exist. Its dependents are skipped
    ManifestBroken {
        /// Why
        reason: &'a str,
    },
    /// A manifest starts
    ManifestStarted {
        /// Its name
        name: &'a str,
    },
    /// The manifest doesn't run. It ends here
    ManifestSkipped(Skip<'a>),
    /// The manifest's `where` was evaluated
    WhereEvaluated {
        /// The condition
        condition: &'a str,
        /// What it gave
        result: bool,
    },
    /// The manifest ran all of its actions, or stopped at one that failed. It ends here
    ManifestDone {
        /// Whether an action failed
        failed: bool,
    },
    /// An action of the manifest starts
    ActionStarted {
        /// The action, as it's shown
        action: &'a str,
    },
    /// The action can't plan its steps. It ends here
    ActionPlanFailed(&'a actions::Error),
    /// A step of the action doesn't run
    StepSkipped {
        /// The step
        step: &'a Step,
        /// Why
        reason: StepSkip<'a>,
    },
    /// What a step changes, like a diff, if its atom can say more than its name. In a dry run,
    /// this is the change it would make
    StepChange(&'a str),
    /// A step starts to make its change. It runs until the next event of this manifest that
    /// isn't a [`Event::StepOutput`]
    StepStarted(&'a Step),
    /// A line that the program of a step printed, while it runs
    StepOutput {
        /// Where it printed it
        stream: Stream,
        /// The line, without its line break
        line: &'a str,
    },
    /// A step failed, the action stops
    StepFailed {
        /// The step
        step: &'a Step,
        /// Why
        error: &'a atoms::Error,
    },
    /// A step's finalizers stop the action
    FinalizersFailed,
    /// The run was asked to stop, e.g. with Ctrl-C. The step that's running finishes, nothing
    /// starts after it
    Cancelled,
    /// The action ran its steps that are needed, or stopped at one that failed. It ends here
    ActionDone {
        /// What the action did, `None` if no step was needed
        summary: Option<String>,
    },
}

/// Why a manifest doesn't run
pub enum Skip<'a> {
    /// A manifest it depends on failed
    DependencyFailed,
    /// It doesn't have the label of `--label`
    LabelMissing(&'a str),
    /// Its `where` is false
    WhereFalse,
    /// Its `where` can't be evaluated, which counts as false
    WhereFailed {
        /// The condition
        condition: &'a str,
        /// Why
        error: &'a anyhow::Error,
    },
}

/// Why a step doesn't run, though it would change something
pub enum StepSkip<'a> {
    /// It needs this path, and nothing makes it
    Missing(&'a Path),
    /// Its atom can't tell whether something changes
    CheckFailed(&'a atoms::Error),
}

/// Receives the events of a run. With `--jobs`, manifests run in threads, and each calls it for
/// its own events from its thread, so the events of a manifest are in order but may come between
/// those of another one
pub trait Observer: Sync {
    /// Called for each event, in the order they happen in its thread
    fn on(&self, event: &Event);
}

impl<F: Fn(&Event) + Sync> Observer for F {
    fn on(&self, event: &Event) {
        self(event)
    }
}

/// Ignores every event
pub struct Ignore;

impl Observer for Ignore {
    fn on(&self, _: &Event) {}
}
