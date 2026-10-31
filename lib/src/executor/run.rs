//! Runs the manifests of a [`ManifestGraph`]: each manifest's actions, each action's steps.

use super::{
    event::{Event, Observer, Skip, StepSkip},
    graph::ManifestGraph,
    locks::Locks,
    schedule, Options, Report,
};
use crate::helper::Helper;
use crate::{
    actions::{Action, PlanCx},
    atoms::{self, command::Stream, Atom, Check, Output},
    contexts::evaluate_condition,
    manifests::Manifest,
    steps::Step,
};
use petgraph::graph::NodeIndex;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Mutex,
    },
};

/// What a step changes, like a diff, if its atom can say more than its name
type Change = Option<String>;

/// One `comtrya apply`: walks the graph and remembers which manifests failed. Shared by the
/// threads of `--jobs`, so what changes during the run is behind a lock or atomic
pub(super) struct Run<'a> {
    /// The manifests and their dependencies
    graph: &'a ManifestGraph,
    /// The facts and contexts for `where` conditions and plans
    cx: PlanCx<'a>,
    /// Only plan and show what would change
    dry_run: bool,
    /// Only manifests with this label run
    label: Option<&'a str>,
    /// How many manifests may run at once, see [`Options::jobs`]
    jobs: usize,
    /// Receives what happens
    observer: &'a dyn Observer,
    /// Set when the run should stop, see [`Options::cancel`]
    cancel: Option<&'a AtomicBool>,
    /// Whether the run stopped because of `cancel`
    cancelled: AtomicBool,
    /// Runs the root steps that aren't programs, started on first use. Shared by the jobs, one
    /// helper per run
    helper: Mutex<Helper>,
    /// Manifests that failed, or were skipped because a dependency failed
    failed: Mutex<HashSet<NodeIndex>>,
    /// What the steps of a dry run would have made so far. A later step that needs one of
    /// these paths would find it in a real run
    produced: Mutex<Vec<PathBuf>>,
    /// What the running steps use, so steps of manifests that run at once take turns
    locks: Locks,
}

impl<'a> Run<'a> {
    /// A run that hasn't visited any manifest yet
    pub(super) fn new(graph: &'a ManifestGraph, cx: PlanCx<'a>, options: &Options<'a>) -> Self {
        Run {
            graph,
            cx,
            dry_run: options.dry_run,
            label: options.label,
            jobs: options.jobs(),
            observer: options.observer,
            cancel: options.cancel,
            cancelled: AtomicBool::new(false),
            helper: Mutex::new(Helper::new(cx.facts.privilege.clone())),
            failed: Mutex::new(HashSet::new()),
            produced: Mutex::new(vec![]),
            locks: Locks::default(),
        }
    }

    /// Runs the manifests in `order`, see [`ManifestGraph::run_order`]. Several at once with
    /// `--jobs`, each after its dependencies
    pub(super) fn all(&self, order: Vec<NodeIndex>) {
        schedule::visit_all(self.graph, order, self.jobs, |node| {
            match self.graph.broken.get(&node) {
                Some(reason) => self.broken(node, reason),
                None => self.visit(node),
            }
        });
    }

    /// Reports a manifest that fails before it runs, and fails it
    fn broken(&self, node: NodeIndex, reason: &str) {
        self.observer.on(&Event::ManifestBroken { reason });
        self.failed().insert(node);
    }

    /// What the run did
    pub(super) fn finish(self) -> Report {
        let mut failed: Vec<String> = self
            .failed
            .into_inner()
            .expect("no manifest panicked while it noted a failure")
            .iter()
            .filter_map(|&node| self.graph.dag[node].name.clone())
            .collect();
        failed.sort();

        Report {
            failed,
            cancelled: self.cancelled.into_inner(),
        }
    }

    /// Runs one manifest, unless it's the root, failed already, or something skips it
    fn visit(&self, node: NodeIndex) {
        let manifest = &self.graph.dag[node];
        // The root has no name. Failed manifests were reported already
        let Some(name) = manifest.name.as_deref() else {
            return;
        };
        if self.failed().contains(&node) || self.stops() {
            return;
        }

        self.observer.on(&Event::ManifestStarted { name });

        if self.graph.depends_on_any(node, &self.failed()) {
            self.observer
                .on(&Event::ManifestSkipped(Skip::DependencyFailed));
            self.failed().insert(node);
            return;
        }

        if !self.has_label(manifest) || !self.where_allows(manifest) {
            return;
        }

        let failed = !self.run_actions(manifest);
        if failed {
            self.failed().insert(node);
        }
        self.observer.on(&Event::ManifestDone { failed });
    }

    /// The manifests that failed so far
    fn failed(&self) -> std::sync::MutexGuard<'_, HashSet<NodeIndex>> {
        self.failed
            .lock()
            .expect("no manifest panicked while it noted a failure")
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
    /// Later actions usually build on earlier ones, so a failure stops the manifest, unless the
    /// action has `continue_on_error`. A dry run changes nothing, so it goes on and reports
    /// every action that can't be planned.
    fn run_actions(&self, manifest: &Manifest) -> bool {
        let mut successful = true;

        for action in &manifest.actions {
            if self.stops() {
                return false;
            }
            let shown = action.to_string();
            let action = action.inner_ref();
            self.observer.on(&Event::ActionStarted {
                action: &shown,
                continue_on_error: action.continue_on_error(),
            });

            // A cancelled run stops at the top of the loop, also after such an action
            if !self.run_action(manifest, action) && !action.continue_on_error() {
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
            if self.stops() {
                successful = false;
                break;
            }
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

    /// Whether the run should stop. Tells it once, the first time
    fn stops(&self) -> bool {
        let asked = self
            .cancel
            .is_some_and(|cancel| cancel.load(Ordering::Relaxed));
        if asked && !self.cancelled.swap(true, Ordering::Relaxed) {
            self.observer.on(&Event::Cancelled);
        }
        asked
    }

    /// Tells that `step` doesn't run
    fn skipped(&self, step: &Step, reason: StepSkip) {
        self.observer.on(&Event::StepSkipped { step, reason });
    }

    /// Whether an earlier step of this dry run would have made `path`, or a directory it's in
    fn will_exist(&self, path: &Path) -> bool {
        self.produced
            .lock()
            .expect("no step panicked while it noted a path")
            .iter()
            .any(|produced| path.starts_with(produced))
    }

    /// Shows what the step changes and, unless this is a dry run, runs it. False if it failed
    fn run_step(&self, step: Step, change: Change) -> bool {
        if let Some(change) = change {
            self.observer.on(&Event::StepChange(&change));
        }

        if self.dry_run {
            self.produced
                .lock()
                .expect("no step panicked while it noted a path")
                .extend(step.atom.produces().map(Path::to_path_buf));
            return true;
        }

        let _lock = self.locks.lock(step.resources());
        let output = match self.run_atom(&step) {
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

    /// Runs `atom`, and tells each line its program prints while it runs. The atom runs in its
    /// own thread, so the lines reach the observer here, in the spans of the manifest and the
    /// action
    fn execute(&self, atom: &Atom) -> atoms::Result<Output> {
        let (sender, lines) = mpsc::channel::<(Stream, String)>();

        std::thread::scope(|scope| {
            let running = scope.spawn(move || {
                atom.execute_with_lines(&move |stream, line| {
                    // The receiver lives until the atom is done
                    let _ = sender.send((stream, line.to_string()));
                })
            });

            for (stream, line) in lines {
                self.observer.on(&Event::StepOutput {
                    stream,
                    line: &line,
                });
            }

            running
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        })
    }

    /// Runs the step's atom: in the privileged helper if it needs root and isn't a program,
    /// otherwise here, through the privilege provider for a root program
    fn run_atom(&self, step: &Step) -> atoms::Result<Output> {
        if step.needs_helper() {
            return self
                .helper
                .lock()
                .expect("no step panicked while it used the helper")
                .run(&step.atom);
        }
        step.atom_to_run(&self.cx.facts.privilege)
            .and_then(|atom| self.execute(&atom))
    }
}
