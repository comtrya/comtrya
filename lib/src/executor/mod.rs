//! Applies manifests: dependencies before the manifests that need them, each manifest's actions
//! in order, and each action's steps that are needed

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

    let run = Run::new(&graph, cx, options);
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
            Event::ManifestBroken { .. } => String::from("broken"),
            Event::ManifestStarted { name } => format!("start {name}"),
            Event::ManifestSkipped(Skip::DependencyFailed) => String::from("skip: dependency"),
            Event::ManifestSkipped(_) => String::from("skip"),
            Event::ManifestDone { failed } => format!("done, failed: {failed}"),
            Event::ActionStarted { action, .. } => format!("action {action}"),
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
        let events = std::sync::Mutex::new(vec![]);
        let record = |event: &Event| events.lock().unwrap().push(name(event));
        let options = Options {
            observer: &record,
            ..Default::default()
        };

        run(loaded, dir.path(), PlanCx::empty(), &options);
        run(loaded_ok, dir.path(), PlanCx::empty(), &options);

        assert_eq!(
            vec![
                "broken",
                "start app",
                "skip: dependency",
                "start ok",
                "action directory.create",
                "action done: true",
                "done, failed: false",
            ],
            events.into_inner().unwrap()
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
        let events = std::sync::Mutex::new(vec![]);
        // Ctrl-C while the action plans, right before its step would run
        let record = |event: &Event| {
            if let Event::ActionStarted { .. } = event {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            events.lock().unwrap().push(name(event));
        };
        let options = Options {
            observer: &record,
            cancel: Some(&cancel),
            ..Default::default()
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &options);

        assert!(report.cancelled);
        assert!(!dir.path().join("app").exists());
        assert!(events.lock().unwrap().contains(&String::from("cancelled")));
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
        let lines = std::sync::Mutex::new(vec![]);
        let record = |event: &Event| {
            if let Event::StepOutput { stream, line } = event {
                lines.lock().unwrap().push(format!("{stream:?} {line}"));
            }
        };
        let options = Options {
            observer: &record,
            ..Default::default()
        };

        run(loaded, dir.path(), PlanCx::empty(), &options);

        let mut lines = lines.into_inner().unwrap();
        lines.sort();
        assert_eq!(vec!["Stderr two", "Stdout one"], lines);
    }

    /// Serves `hello` on localhost. Each answer waits until `together` requests are open at
    /// once, up to a second, else it fails with 500. Returns the URL, and how many requests
    /// were open at once at most
    fn serve(together: usize) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::io::{Read, Write};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let open = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let most_seen = Arc::clone(&most);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let (open, most) = (Arc::clone(&open), Arc::clone(&most));
                std::thread::spawn(move || {
                    let mut stream = stream.unwrap();
                    let _ = stream.read(&mut [0; 1024]);
                    let now = open.fetch_add(1, Ordering::SeqCst) + 1;
                    most.fetch_max(now, Ordering::SeqCst);
                    let deadline = Instant::now() + Duration::from_secs(1);
                    while open.load(Ordering::SeqCst) < together && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    let status = match open.load(Ordering::SeqCst) >= together {
                        true => "200 OK",
                        false => "500 Alone",
                    };
                    // Keep this request open a moment, so overlapping ones are seen
                    std::thread::sleep(Duration::from_millis(200));
                    let answer = format!(
                        "HTTP/1.1 {status}\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello"
                    );
                    let _ = stream.write_all(answer.as_bytes());
                    open.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
        (url, most_seen)
    }

    /// A manifest `name` that downloads `url` to `to`
    fn download(name: &str, url: &str, to: &Path) -> (String, Manifest) {
        let yaml = format!(
            "actions:\n  - action: file.download\n    from: '{url}'\n    to: '{}'\n",
            to.display()
        );
        let manifest = Manifest {
            name: Some(name.to_string()),
            ..serde_yaml_ng::from_str(&yaml).unwrap()
        };
        (name.to_string(), manifest)
    }

    /// Runs `manifests` with `jobs`
    fn run_jobs(manifests: Vec<(String, Manifest)>, dir: &Path, jobs: usize) -> Report {
        let loaded = Loaded {
            manifests: manifests.into_iter().collect(),
            errors: vec![],
        };
        let options = Options {
            jobs,
            ..Default::default()
        };
        run(loaded, dir, PlanCx::empty(), &options)
    }

    #[test]
    fn with_jobs_independent_manifests_run_at_the_same_time() {
        let dir = tempfile::tempdir().unwrap();
        let (url, _) = serve(2);
        let manifests = vec![
            download("a", &url, &dir.path().join("a")),
            download("b", &url, &dir.path().join("b")),
        ];

        // Each download only succeeds while the other one is open too
        let report = run_jobs(manifests, dir.path(), 2);

        assert_eq!(Report::default(), report);
        assert!(dir.path().join("a").is_file() && dir.path().join("b").is_file());
    }

    #[test]
    fn one_job_runs_one_manifest_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let (url, most) = serve(1);
        let manifests = vec![
            download("a", &url, &dir.path().join("a")),
            download("b", &url, &dir.path().join("b")),
        ];

        run_jobs(manifests, dir.path(), 1);

        assert_eq!(1, most.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn steps_that_write_the_same_file_take_turns() {
        let dir = tempfile::tempdir().unwrap();
        let (url, most) = serve(1);
        let same = dir.path().join("same");
        let manifests = vec![download("a", &url, &same), download("b", &url, &same)];

        let report = run_jobs(manifests, dir.path(), 2);

        assert_eq!(Report::default(), report);
        assert_eq!(1, most.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn a_broken_manifest_that_isnt_selected_doesnt_fail_the_run() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = Loaded {
            manifests: HashMap::from([manifest("app", &[], &dir.path().join("app"))]),
            errors: vec![broken("broken")],
        };
        let selected = [String::from("app")];
        let options = Options {
            manifests: &selected,
            ..Default::default()
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &options);

        assert_eq!(Report::default(), report);
        assert!(dir.path().join("app").is_dir());
    }

    /// A manifest `name` from `yaml`
    fn manifest_from(name: &str, yaml: &str) -> (String, Manifest) {
        let manifest = Manifest {
            name: Some(name.to_string()),
            ..serde_yaml_ng::from_str(yaml).unwrap()
        };
        (name.to_string(), manifest)
    }

    #[cfg(unix)]
    #[test]
    fn continue_on_error_goes_on_with_the_next_action() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = format!(
            "actions:\n  - action: command.run\n    command: 'false'\n    continue_on_error: true\n  - action: directory.create\n    path: '{}'\n",
            dir.path().join("after").display()
        );
        let loaded = Loaded {
            manifests: HashMap::from([
                manifest_from("tolerant", &yaml),
                manifest("dependent", &["tolerant"], &dir.path().join("dependent")),
            ]),
            errors: vec![],
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

        assert_eq!(Report::default(), report);
        assert!(dir.path().join("after").is_dir());
        assert!(dir.path().join("dependent").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn without_continue_on_error_a_failure_stops_the_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = format!(
            "actions:\n  - action: command.run\n    command: 'false'\n  - action: directory.create\n    path: '{}'\n",
            dir.path().join("after").display()
        );
        let loaded = Loaded {
            manifests: HashMap::from([manifest_from("strict", &yaml)]),
            errors: vec![],
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &Options::default());

        assert_eq!(vec!["strict"], report.failed);
        assert!(!dir.path().join("after").exists());
    }

    #[test]
    fn continue_on_error_doesnt_go_on_after_a_cancel() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = format!(
            "actions:\n  - action: directory.create\n    path: '{0}/a'\n    continue_on_error: true\n  - action: directory.create\n    path: '{0}/b'\n",
            dir.path().display()
        );
        let loaded = Loaded {
            manifests: HashMap::from([manifest_from("tolerant", &yaml)]),
            errors: vec![],
        };
        let cancel = AtomicBool::new(false);
        // Ctrl-C while the first action plans: its step and everything after it don't run
        let record = |event: &Event| {
            if let Event::ActionStarted { .. } = event {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        };
        let options = Options {
            observer: &record,
            cancel: Some(&cancel),
            ..Default::default()
        };

        let report = run(loaded, dir.path(), PlanCx::empty(), &options);

        assert!(report.cancelled);
        assert!(!dir.path().join("a").exists());
        assert!(!dir.path().join("b").exists());
    }
}
