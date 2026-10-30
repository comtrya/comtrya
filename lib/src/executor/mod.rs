//! Applies manifests: dependencies before the manifests that need them, each manifest's actions
//! in order, and each action's steps that are needed

/// The manifests and their dependencies as a graph
mod graph;
/// Walking the graph and running the steps
mod run;

use crate::{actions::PlanCx, manifests::Loaded};
use graph::ManifestGraph;
use run::Run;
use std::path::Path;

/// What to run, and how
#[derive(Clone, Copy, Debug, Default)]
pub struct Options<'a> {
    /// Only these manifests, with their dependencies. All of them if empty
    pub manifests: &'a [String],
    /// Only plan and show what would change
    pub dry_run: bool,
    /// Only manifests with this label
    pub label: Option<&'a str>,
}

/// What a run did
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The manifests that failed, couldn't be loaded, or were skipped because a dependency
    /// failed, by name
    pub failed: Vec<String>,
}

impl Report {
    /// An error if any manifest failed
    pub fn into_result(self) -> anyhow::Result<()> {
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

    let mut run = Run::new(&graph, cx, options.dry_run, options.label);
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
}
