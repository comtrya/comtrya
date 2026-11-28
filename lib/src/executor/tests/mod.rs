//! Tests of the executor: manifests built from YAML, run in a temporary directory

mod cancel;
mod events;
mod failures;
mod jobs;
mod options;
// The commands are `sh`
#[cfg(unix)]
mod registered;

use super::*;
use crate::manifests::{LoadError, LoadErrorKind, Manifest};
use std::path::PathBuf;

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

/// A manifest `name` that failed to load
fn broken(name: &str) -> LoadError {
    LoadError {
        name: name.to_string(),
        path: PathBuf::from(format!("{name}.yaml")),
        kind: LoadErrorKind::Parse(String::from("nope")),
    }
}

/// A manifest `name` from `yaml`
fn manifest_from(name: &str, yaml: &str) -> (String, Manifest) {
    let manifest = Manifest {
        name: Some(name.to_string()),
        ..serde_yaml_ng::from_str(yaml).unwrap()
    };
    (name.to_string(), manifest)
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
        Event::StepStarted(_) => String::from("step"),
        Event::Cancelled => String::from("cancelled"),
        _ => String::from("other"),
    }
}
