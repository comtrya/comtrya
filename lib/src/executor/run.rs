//! Runs the manifests of a [`ManifestGraph`]: each manifest's actions, each action's steps.

use super::{
    event::{Event, Observer, Skip, StepSkip},
    graph::ManifestGraph,
    Options, Report,
};
use crate::{
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

/// What a step changes, like a diff, if its atom can say more than its name
type Change = Option<String>;

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
    /// Receives what happens
    observer: &'a dyn Observer,
    /// Manifests that failed, or were skipped because a dependency failed
    failed: HashSet<NodeIndex>,
    /// What the steps of a dry run would have made so far. A later step that needs one of
    /// these paths would find it in a real run
    produced: Vec<PathBuf>,
}

impl<'a> Run<'a> {
    /// A run that hasn't visited any manifest yet
    pub(super) fn new(graph: &'a ManifestGraph, cx: PlanCx<'a>, options: &Options<'a>) -> Self {
        Run {
            graph,
            cx,
            dry_run: options.dry_run,
            label: options.label,
            observer: options.observer,
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

    /// What the run did
    pub(super) fn finish(self) -> Report {
        let mut failed: Vec<String> = self
            .failed
            .iter()
            .filter_map(|&node| self.graph.dag[node].name.clone())
            .collect();
        failed.sort();

        Report { failed }
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

        self.observer.on(&Event::ManifestStarted { name });

        if self.graph.depends_on_any(node, &self.failed) {
            self.observer
                .on(&Event::ManifestSkipped(Skip::DependencyFailed));
            self.failed.insert(node);
            return;
        }

        if !self.has_label(manifest) || !self.where_allows(manifest) {
            return;
        }

        let failed = !self.run_actions(manifest);
        if failed {
            self.failed.insert(node);
        }
        self.observer.on(&Event::ManifestDone { failed });
    }

    /// Whether the manifest has the label of `--label`, if one was given
    fn has_label(&self, manifest: &Manifest) -> bool {
        let Some(label) = self.label else {
            return true;
        };

        let has_label = manifest.labels.iter().any(|l| l == label);
        if !has_label {
            self.observer
                .on(&Event::ManifestSkipped(Skip::LabelMissing(label)));
        }
        has_label
    }

    /// Whether the manifest's `where` condition, if it has one, is true.
    /// A condition that can't be evaluated counts as false
    fn where_allows(&self, manifest: &Manifest) -> bool {
        let Some(condition) = &manifest.r#where else {
            return true;
        };

        match evaluate_condition(condition, self.cx.contexts) {
            Ok(result) => {
                self.observer
                    .on(&Event::WhereEvaluated { condition, result });
                if !result {
                    self.observer.on(&Event::ManifestSkipped(Skip::WhereFalse));
                }
                result
            }
            Err(error) => {
                let skip = Skip::WhereFailed {
                    condition,
                    error: &error,
                };
                self.observer.on(&Event::ManifestSkipped(skip));
                false
            }
        }
    }

    /// Runs the manifest's actions in order, false if any failed.
    ///
    /// Later actions usually build on earlier ones, so a failure stops the manifest. A dry run
    /// changes nothing, so it goes on and reports every action that can't be planned.
    fn run_actions(&mut self, manifest: &Manifest) -> bool {
        let mut successful = true;

        for action in &manifest.actions {
            let shown = action.to_string();
            self.observer.on(&Event::ActionStarted { action: &shown });

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
            Err(error) => {
                self.observer.on(&Event::ActionPlanFailed(&error));
                return false;
            }
        };

        // Each step is checked right before it runs: whether it's needed may depend on the
        // steps before it, like a link that needs the directory an earlier step creates
        let mut ran_any = false;
        let mut successful = true;
        for step in steps {
            let Some(change) = self.needed_change(&step) else {
                continue;
            };
            ran_any = true;
            if !self.run_step(step, change) {
                successful = false;
                break;
            }
        }

        let summary = ran_any.then(|| action.summarize());
        self.observer.on(&Event::ActionDone { summary });
        successful
    }

    /// `Some` if a step has to run: its initializers allow it, and its atom finds something to
    /// change. Inside is what it changes, if the atom can say. An atom that can't tell, or
    /// can't run, is skipped with a warning
    fn needed_change(&self, step: &Step) -> Option<Change> {
        if !step.do_initializers_allow_us_to_run() {
            return None;
        }

        match step.atom.check() {
            Ok(Check::InSync) => None,
            Ok(Check::Drift(change)) => Some(change),
            Ok(Check::Pending { needs }) if self.will_exist(&needs) => Some(None),
            Ok(Check::Pending { needs }) => {
                self.skipped(step, StepSkip::Missing(&needs));
                None
            }
            Err(error) => {
                self.skipped(step, StepSkip::CheckFailed(&error));
                None
            }
        }
    }

    /// Tells that `step` doesn't run
    fn skipped(&self, step: &Step, reason: StepSkip) {
        self.observer.on(&Event::StepSkipped { step, reason });
    }

    /// Whether an earlier step of this dry run would have made `path`, or a directory it's in
    fn will_exist(&self, path: &Path) -> bool {
        self.produced
            .iter()
            .any(|produced| path.starts_with(produced))
    }

    /// Shows what the step changes and, unless this is a dry run, runs it. False if it failed
    fn run_step(&mut self, step: Step, change: Change) -> bool {
        if let Some(change) = change {
            self.observer.on(&Event::StepChange(&change));
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
            Err(error) => {
                self.observer.on(&Event::StepFailed {
                    step: &step,
                    error: &error,
                });
                return false;
            }
        };

        if !step.do_finalizers_allow_us_to_continue(&output) {
            self.observer.on(&Event::FinalizersFailed);
            return false;
        }

        true
    }
}
