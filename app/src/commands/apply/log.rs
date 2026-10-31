//! Shows the events of a run as log lines, inside a span for the manifest and one for the action.
//! With `--jobs`, each manifest runs in a thread of its own, and tracing's spans belong to a
//! thread, so each thread keeps its own spans: every line names its manifest and action, also
//! when lines of several manifests come between each other

use super::progress::Progress;
use comtrya_lib::{
    atoms::{command::Stream, Resource, RunAs},
    executor::{Event, Observer, Skip, StepSkip},
    steps::Step,
};
use std::cell::RefCell;
use tracing::{debug, error, info, span::EnteredSpan, warn};

thread_local! {
    /// The spans of the manifest and the action that run in this thread, innermost last
    static SPANS: RefCell<Vec<EnteredSpan>> = const { RefCell::new(vec![]) };
}

/// Logs each event of a run with tracing
pub(super) struct Log {
    /// A dry run shows what steps would change, a real run only with `-v`
    dry_run: bool,
    /// The status line for a step that takes a while, if there is one, see [`Progress`]
    progress: Option<Progress>,
}

impl Log {
    /// Logs a run, a dry run if `dry_run`, with a status line for long steps if `progress`
    pub(super) fn new(dry_run: bool, progress: Option<Progress>) -> Self {
        Self { dry_run, progress }
    }

    /// Shows the status line for `step` while it runs, unless it may ask for a password: the
    /// line would be drawn over the prompt
    fn started(&self, step: &Step) {
        let Some(progress) = &self.progress else {
            return;
        };
        let prompts = step.run_as == RunAs::Root || step.resources().contains(&Resource::Tty);
        if !prompts {
            progress.start(&step.to_string());
        }
    }

    /// Lines of this thread after this one belong to `span`, until [`Log::leave`]
    fn enter(&self, span: tracing::Span) {
        SPANS.with_borrow_mut(|spans| spans.push(span.entered()));
    }

    /// Ends the innermost span of this thread
    fn leave(&self) {
        SPANS.with_borrow_mut(|spans| spans.pop());
    }

    /// Why a manifest is skipped
    fn skipped(&self, skip: &Skip) {
        match skip {
            Skip::DependencyFailed => warn!("Skipping manifest, because a dependency failed"),
            Skip::LabelMissing(label) => {
                info!(message = "Skipping manifest, label not found", label)
            }
            Skip::WhereFalse => info!("Skip manifest, because 'where' conditions were false!"),
            Skip::WhereFailed { condition, error } => {
                warn!("'where' condition '{condition}' failed: {error}");
                info!("Skip manifest, because 'where' conditions were false!");
            }
        }
    }
}

/// The error and its causes, like `{:#}` of [`anyhow::Error`]
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    text
}

impl Observer for Log {
    fn on(&self, event: &Event) {
        // Any other event means the step ended; the line goes before the next log line does
        if let (Some(progress), false) = (&self.progress, matches!(event, Event::StepOutput { .. }))
        {
            progress.stop();
        }

        match event {
            Event::StepStarted(step) => self.started(step),
            Event::ManifestBroken { reason } => error!("{reason}"),
            Event::ManifestStarted { name } => {
                self.enter(tracing::info_span!("", manifest = name));
            }
            Event::ManifestSkipped(skip) => {
                self.skipped(skip);
                self.leave();
            }
            Event::WhereEvaluated { condition, result } => {
                debug!("Result of 'where' condition '{condition}' -> '{result}'")
            }
            Event::ManifestDone { failed: true } => {
                error!("Failed");
                self.leave();
            }
            Event::ManifestDone { failed: false } => {
                if !self.dry_run {
                    info!("Completed");
                }
                self.leave();
            }
            Event::ActionStarted { action } => {
                self.enter(tracing::info_span!("", action = %action));
            }
            Event::ActionPlanFailed(error) => {
                error!("Action failed to get plan: {}", chain(*error));
                self.leave();
            }
            Event::StepSkipped { step, reason } => match reason {
                StepSkip::Missing(needs) => {
                    warn!("Skipping {step}: {} doesn't exist", needs.display())
                }
                StepSkip::CheckFailed(error) => {
                    warn!("Skipping {step}: {}", chain(*error))
                }
            },
            Event::StepChange(change) if self.dry_run => info!("{change}"),
            Event::StepChange(change) => debug!("{change}"),
            Event::StepOutput {
                stream: Stream::Stdout,
                line,
            } => debug!("stdout: {line}"),
            Event::StepOutput {
                stream: Stream::Stderr,
                line,
            } => debug!("stderr: {line}"),
            Event::StepFailed { step, error } => {
                error!("{step} failed: {}", chain(*error))
            }
            Event::FinalizersFailed => {
                error!("Finalizers won't allow us to continue with this action")
            }
            Event::Cancelled => warn!("Cancelled, nothing else runs"),
            Event::ActionDone { summary } => {
                match summary {
                    Some(summary) => info!("{summary}"),
                    None => info!("nothing to be done to reconcile action"),
                }
                self.leave();
            }
        }
    }
}
