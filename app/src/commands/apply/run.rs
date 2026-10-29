//! Runs the manifests of a [`ManifestGraph`]: each manifest's actions, each action's steps.

use super::graph::ManifestGraph;
use comtrya_lib::{
    actions::{Action, PlanCx},
    contexts::evaluate_condition,
    manifests::Manifest,
    steps::Step,
};
use petgraph::graph::NodeIndex;
use std::collections::HashSet;
use tracing::{debug, error, info, span, warn};

/// One `comtrya apply`: walks the graph and remembers which manifests failed
pub(super) struct Run<'a> {
    /// The manifests and their dependencies
    graph: &'a ManifestGraph,
    /// The facts and contexts for `where` conditions and plans
    cx: PlanCx<'a>,
    /// Only plan and show what would change
    dry_run: bool,
    /// Only manifests with this label run
    label: Option<&'a str>,
    /// Manifests that failed, or were skipped because a dependency failed
    failed: HashSet<NodeIndex>,
}

impl<'a> Run<'a> {
    /// A run that hasn't visited any manifest yet
    pub(super) fn new(
        graph: &'a ManifestGraph,
        cx: PlanCx<'a>,
        dry_run: bool,
        label: Option<&'a str>,
    ) -> Self {
        Run {
            graph,
            cx,
            dry_run,
            label,
            failed: graph.broken.clone(),
        }
    }

    /// Runs the manifests in `order`, see [`ManifestGraph::run_order`]
    pub(super) fn all(&mut self, order: Vec<NodeIndex>) {
        for node in order {
            self.visit(node);
        }
    }

    /// The result of the run: an error if any manifest failed or was skipped because of one
    pub(super) fn finish(self) -> anyhow::Result<()> {
        if self.failed.is_empty() {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "{} manifest(s) failed or were skipped because a dependency failed",
            self.failed.len()
        ))
    }

    /// Runs one manifest, unless it's the root, failed already, or something skips it
    fn visit(&mut self, node: NodeIndex) {
        let manifest = &self.graph.dag[node];
        // The root has no name. Failed manifests were reported already
        let Some(name) = manifest.name.as_deref() else {
            return;
        };
        if self.failed.contains(&node) {
            return;
        }

        let _span = span!(tracing::Level::INFO, "", manifest = name).entered();

        if self.graph.depends_on_any(node, &self.failed) {
            warn!("Skipping manifest, because a dependency failed");
            self.failed.insert(node);
            return;
        }

        if !self.has_label(manifest) || !self.where_allows(manifest) {
            return;
        }

        if !self.run_actions(manifest) {
            error!("Failed");
            self.failed.insert(node);
        } else if !self.dry_run {
            info!("Completed");
        }
    }

    /// Whether the manifest has the label of `--label`, if one was given
    fn has_label(&self, manifest: &Manifest) -> bool {
        let Some(label) = self.label else {
            return true;
        };

        let has_label = manifest.labels.iter().any(|l| l == label);
        if !has_label {
            info!(message = "Skipping manifest, label not found", label);
        }
        has_label
    }

    /// Whether the manifest's `where` condition, if it has one, is true.
    /// A condition that can't be evaluated counts as false
    fn where_allows(&self, manifest: &Manifest) -> bool {
        let Some(condition) = &manifest.r#where else {
            return true;
        };

        let allowed = match evaluate_condition(condition, self.cx.contexts) {
            Ok(result) => {
                debug!("Result of 'where' condition '{condition}' -> '{result}'");
                result
            }
            Err(err) => {
                warn!("'where' condition '{condition}' failed: {err}");
                false
            }
        };

        if !allowed {
            info!("Skip manifest, because 'where' conditions were false!");
        }
        allowed
    }

    /// Runs the manifest's actions in order, false if any failed.
    ///
    /// Later actions usually build on earlier ones, so a failure stops the manifest. A dry run
    /// changes nothing, so it goes on and reports every action that can't be planned.
    fn run_actions(&self, manifest: &Manifest) -> bool {
        let mut successful = true;

        for action in &manifest.actions {
            let _span = span!(tracing::Level::INFO, "", %action).entered();

            if !self.run_action(manifest, action.inner_ref()) {
                successful = false;
                if !self.dry_run {
                    break;
                }
            }
        }

        successful
    }

    /// Plans one action and runs the steps that are needed, false if it failed
    fn run_action(&self, manifest: &Manifest, action: &dyn Action) -> bool {
        let steps = match action.plan(manifest, &self.cx) {
            Ok(steps) => steps,
            Err(err) => {
                error!("Action failed to get plan: {err:#}");
                return false;
            }
        };

        // Lazy on purpose: whether a step is needed may depend on the steps before it, like a
        // link that needs the directory an earlier step creates
        let mut needed = steps.into_iter().filter(is_needed).peekable();
        if needed.peek().is_none() {
            info!("nothing to be done to reconcile action");
            return true;
        }

        // `all` stops at the first step that fails
        let successful = needed.all(|step| self.run_step(step));
        info!("{}", action.summarize());
        successful
    }

    /// Shows what the step changes and, unless this is a dry run, runs it. False if it failed
    fn run_step(&self, mut step: Step) -> bool {
        if let Some(change) = step.atom.describe_change() {
            if self.dry_run {
                info!("{change}");
            } else {
                debug!("{change}");
            }
        }

        if self.dry_run {
            return true;
        }

        if let Err(err) = step.atom.execute() {
            error!("{} failed: {err:#}", step.atom);
            return false;
        }

        if !step.do_finalizers_allow_us_to_continue() {
            error!("Finalizers won't allow us to continue with this action");
            return false;
        }

        true
    }
}

/// Whether a step has to run: its initializers allow it, and its atom finds something to change.
/// An atom that can't tell is skipped with a warning
fn is_needed(step: &Step) -> bool {
    if !step.do_initializers_allow_us_to_run() {
        return false;
    }

    match step.atom.plan() {
        Ok(outcome) => outcome.should_run,
        Err(err) => {
            warn!("Skipping {}: {err:#}", step.atom);
            false
        }
    }
}
