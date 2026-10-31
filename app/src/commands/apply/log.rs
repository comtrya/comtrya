//! Shows the events of a run as log lines, inside a span for the manifest and one for the action.
//! With `--jobs`, each manifest runs in a thread of its own, and tracing's spans belong to a
//! thread, so each thread keeps its own spans: every line names its manifest and action, also
//! when lines of several manifests come between each other

use comtrya_lib::{
    atoms::command::Stream,
    executor::{Event, Observer, Skip, StepSkip},
};
use std::cell::{Cell, RefCell};
use tracing::{debug, error, info, span::EnteredSpan, warn};

thread_local! {
    /// The spans of the manifest and the action that run in this thread, innermost last
    static SPANS: RefCell<Vec<EnteredSpan>> = const { RefCell::new(vec![]) };

    /// Whether the action that runs in this thread has `continue_on_error`, so its failure is a
    /// warning
    static CONTINUES: Cell<bool> = const { Cell::new(false) };
}

/// Logs each event of a run with tracing
pub(super) struct Log {
    /// A dry run shows what steps would change, a real run only with `-v`
    dry_run: bool,
}

impl Log {
    /// Logs a run, a dry run if `dry_run`
    pub(super) fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    /// Lines of this thread after this one belong to `span`, until [`Log::leave`]
    fn enter(&self, span: tracing::Span) {
        SPANS.with_borrow_mut(|spans| spans.push(span.entered()));
    }

    /// Ends the innermost span of this thread
    fn leave(&self) {
        SPANS.with_borrow_mut(|spans| spans.pop());
    }

    /// Logs that the action failed: an error, or a warning if the manifest goes on
    fn failed(&self, message: &str) {
        if CONTINUES.get() {
            warn!("{message}, continuing (continue_on_error)");
        } else {
            error!("{message}");
        }
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
        match event {
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
            Event::ActionStarted {
                action,
                continue_on_error,
            } => {
                CONTINUES.set(*continue_on_error);
                self.enter(tracing::info_span!("", action = %action));
            }
            Event::ActionPlanFailed(error) => {
                self.failed(&format!("Action failed to get plan: {}", chain(*error)));
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
                self.failed(&format!("{step} failed: {}", chain(*error)))
            }
            Event::FinalizersFailed => {
                self.failed("Finalizers won't allow us to continue with this action")
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
