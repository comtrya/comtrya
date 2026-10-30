//! Runs the manifests of a [`ManifestGraph`]: each manifest's actions, each action's steps.

use super::graph::ManifestGraph;
use comtrya_lib::{
    actions::{Action, PlanCx},
    atoms::Check,
    contexts::evaluate_condition,
    manifests::Manifest,
    steps::Step,
};
use petgraph::graph::NodeIndex;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
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
    /// What the steps of a dry run would have made so far. A later step that needs one of
    /// these paths would find it in a real run
    produced: Vec<PathBuf>,
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
            produced: vec![],
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
    fn run_actions(&mut self, manifest: &Manifest) -> bool {
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
    fn run_action(&mut self, manifest: &Manifest, action: &dyn Action) -> bool {
        let steps = match action.plan(manifest, &self.cx) {
            Ok(steps) => steps,
            Err(err) => {
                error!("Action failed to get plan: {err:#}");
                return false;
            }
        };

        // Each step is checked right before it runs: whether it's needed may depend on the
        // steps before it, like a link that needs the directory an earlier step creates
        let mut ran_any = false;
        let mut successful = true;
        for step in steps {
            if !self.is_needed(&step) {
                continue;
            }
            ran_any = true;
            if !self.run_step(step) {
                successful = false;
                break;
            }
        }

        if !ran_any {
            info!("nothing to be done to reconcile action");
            return true;
        }
        info!("{}", action.summarize());
        successful
    }

    /// Whether a step has to run: its initializers allow it, and its atom finds something to
    /// change. An atom that can't tell, or can't run, is skipped with a warning
    fn is_needed(&self, step: &Step) -> bool {
        if !step.do_initializers_allow_us_to_run() {
            return false;
        }

        match step.atom.check() {
            Ok(Check::InSync) => false,
            Ok(Check::Drift) => true,
            Ok(Check::Pending { needs }) if self.will_exist(&needs) => true,
            Ok(Check::Pending { needs }) => {
                warn!("Skipping {step}: {} doesn't exist", needs.display());
                false
            }
            Err(err) => {
                warn!("Skipping {step}: {err:#}");
                false
            }
        }
    }

    /// Whether an earlier step of this dry run would have made `path`, or a directory it's in
    fn will_exist(&self, path: &Path) -> bool {
        self.produced
            .iter()
            .any(|produced| path.starts_with(produced))
    }

    /// Shows what the step changes and, unless this is a dry run, runs it. False if it failed
    fn run_step(&mut self, step: Step) -> bool {
        if let Some(change) = step.atom.describe_change() {
            if self.dry_run {
                info!("{change}");
            } else {
                debug!("{change}");
            }
        }

        if self.dry_run {
            self.produced
                .extend(step.atom.produces().map(Path::to_path_buf));
            return true;
        }

        let output = match step
            .atom_to_run(&self.cx.facts.privilege)
            .and_then(|atom| atom.execute())
        {
            Ok(output) => output,
            Err(err) => {
                error!("{step} failed: {err:#}");
                return false;
            }
        };

        if !step.do_finalizers_allow_us_to_continue(&output) {
            error!("Finalizers won't allow us to continue with this action");
            return false;
        }

        true
    }
}
