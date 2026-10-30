//! Applies manifests: dependencies before the manifests that need them, each manifest's actions
//! in order, and each action's steps that are needed

/// What the executor tells about a run
mod event;
/// The manifests and their dependencies as a graph
mod graph;
/// Walking the graph and running the steps
mod run;

use crate::{actions::PlanCx, manifests::Loaded};
pub use event::{Event, Ignore, Observer, Skip, StepSkip};
use graph::ManifestGraph;
use run::Run;
use std::{path::Path, sync::atomic::AtomicBool};

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

impl Report {
    /// An error if any manifest failed, or the run was cancelled
    pub fn into_result(self) -> anyhow::Result<()> {
        if self.cancelled {
            return Err(anyhow::anyhow!("Cancelled"));
        }
        if self.failed.is_empty() {
            return Ok(());
        }

        Err(anyhow::anyhow!(
            "{} manifest(s) failed or were skipped because a dependency failed",
            self.failed.len()
        ))
    }
}

/// Runs the manifests loaded from `manifest_dir`. A manifest that fails stops only itself and
/// the manifests that depend on it
pub fn run(loaded: Loaded, manifest_dir: &Path, cx: PlanCx, options: &Options) -> Report {
    let graph = ManifestGraph::build(loaded, manifest_dir);

    let mut run = Run::new(&graph, cx, options);
    run.all(graph.run_order(options.manifests));
    run.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::{LoadError, LoadErrorKind, Manifest};
    use pretty_assertions::assert_eq;
    use std::{collections::HashMap, path::PathBuf};

    /// A manifest `name` that creates `directory`, after `depends`
    fn manifest(name: &str, depends: &[&str], directory: &Path) -> (String, Manifest) {
        let yaml = format!(
            "depends: {depends:?}\nactions:\n  - action: directory.create\n    path: '{}'\n",
            directory.display()
        );
        let manifest = Manifest {
            name: Some(name.to_string()),
            ..serde_yaml_ng::from_str(&yaml).unwrap()
        };
        (name.to_string(), manifest)
    }

    fn broken(name: &str) -> LoadError {
        LoadError {
            name: name.to_string(),
            path: PathBuf::from(format!("{name}.yaml")),
            kind: LoadErrorKind::Parse(String::from("nope")),
        }
    }

    #[test]
    fn a_failure_stops_only_the_manifests_that_depend_on_it() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Loaded {
            manifests: HashMap::from([
                manifest("app", &["broken"], &dir.path().join("app")),
                manifest("other", &[], &dir.path().join("other")),
            ]),
            errors: vec![broken("broken")],
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

        assert_eq!(vec!["app", "broken"], report.failed);
        assert!(!dir.path().join("app").exists());
        assert!(dir.path().join("other").is_dir());
        assert!(report.into_result().is_err());
    }

    #[test]
    fn a_dry_run_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Loaded {
            manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
            errors: vec![],
        };
        let options = Options {
            dry_run: true,
            ..Default::default()
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &options);

        assert_eq!(Report::default(), report);
        assert!(!dir.path().join("app").exists());
    }

    /// A short name for each event, to compare the order they come in
    fn name(event: &Event) -> String {
        match event {
            Event::ManifestStarted { name } => format!("start {name}"),
            Event::ManifestSkipped(Skip::DependencyFailed) => String::from("skip: dependency"),
            Event::ManifestSkipped(_) => String::from("skip"),
            Event::ManifestDone { failed } => format!("done, failed: {failed}"),
            Event::ActionStarted { action } => format!("action {action}"),
            Event::ActionDone { summary } => format!("action done: {}", summary.is_some()),
            Event::StepChange(_) => String::from("change"),
            Event::Cancelled => String::from("cancelled"),
            _ => String::from("other"),
        }
    }

    #[test]
    fn events_tell_what_happens_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Loaded {
            manifests: HashMap::from([manifest("app", &["broken"], &dir.path().join("app"))]),
            errors: vec![broken("broken")],
        };
        let loaded_ok = Loaded {
            manifests: HashMap::from([manifest("ok", &[], &dir.path().join("ok"))]),
            errors: vec![],
        };
        let events = std::cell::RefCell::new(vec![]);
        let record = |event: &Event| events.borrow_mut().push(name(event));
        let options = Options {
            observer: &record,
            ..Default::default()
        };

        run(loaded, dir.path(), PlanCx::empty(), &options);
        run(loaded_ok, dir.path(), PlanCx::empty(), &options);

        assert_eq!(
            vec![
                "start app",
                "skip: dependency",
                "start ok",
                "action directory.create",
                "action done: true",
                "done, failed: false",
            ],
            events.into_inner()
        );
    }

    #[test]
    fn a_cancel_stops_before_the_next_step() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Loaded {
            manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
            errors: vec![],
        };
        let cancel = AtomicBool::new(false);
        let events = std::cell::RefCell::new(vec![]);
        // Ctrl-C while the action plans, right before its step would run
        let record = |event: &Event| {
            if let Event::ActionStarted { .. } = event {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            events.borrow_mut().push(name(event));
        };
        let options = Options {
            observer: &record,
            cancel: Some(&cancel),
            ..Default::default()
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &options);

        assert!(report.cancelled);
        assert!(!dir.path().join("app").exists());
        assert!(events.borrow().contains(&String::from("cancelled")));
        assert!(report.into_result().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn lines_a_program_prints_are_events() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "actions:\n  - action: command.run\n    command: sh\n    args: [-c, 'echo one; echo two >&2']\n";
        let manifest = Manifest {
            name: Some(String::from("echo")),
            ..serde_yaml_ng::from_str(yaml).unwrap()
        };
        let loaded = Loaded {
            manifests: HashMap::from([(String::from("echo"), manifest)]),
            errors: vec![],
        };
        let lines = std::cell::RefCell::new(vec![]);
        let record = |event: &Event| {
            if let Event::StepOutput { stream, line } = event {
                lines.borrow_mut().push(format!("{stream:?} {line}"));
            }
        };
        let options = Options {
            observer: &record,
            ..Default::default()
        };

        run(loaded, dir.path(), PlanCx::empty(), &options);

        let mut lines = lines.into_inner();
        lines.sort();
        assert_eq!(vec!["Stderr two", "Stdout one"], lines);
    }
}
