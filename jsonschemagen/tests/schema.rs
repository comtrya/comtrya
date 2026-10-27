//! Checks the generated schema

use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the binary like the mdBook workflow does, so anything else printed to stdout breaks the test
fn generate_schema() -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_jsonschemagen"))
        .output()
        .expect("jsonschemagen should run");
    assert!(output.status.success());

    String::from_utf8(output.stdout).expect("schema should be UTF-8")
}

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
}

#[test]
fn schema_is_valid_json() {
    let schema: serde_json::Value =
        serde_json::from_str(&generate_schema()).expect("stdout should only contain the schema");

    assert_eq!("Manifest", schema["title"]);
}

#[test]
fn schema_is_independent_of_the_generating_machine() {
    let schema = generate_schema();

    let cwd = std::env::current_dir().unwrap();
    assert!(
        !schema.contains(&cwd.display().to_string()),
        "schema contains the current directory"
    );
    if let Some(home) = std::env::var_os("HOME") {
        assert!(
            !schema.contains(&*home.to_string_lossy()),
            "schema contains the home directory"
        );
    }
}

/// Examples that comtrya accepts but the schema rejects, or the other way round
const KNOWN_MISMATCHES: [&str; 0] = [];

#[test]
fn examples_match_the_schema() {
    let schema: serde_json::Value = serde_json::from_str(&generate_schema()).unwrap();
    let validator =
        jsonschema::validator_for(&schema).expect("schema should be a valid JSON schema");

    let examples = examples_dir();
    let mut checked = 0;
    let mut failures = vec![];

    for entry in walkdir::WalkDir::new(&examples)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let relative = path.strip_prefix(&examples).unwrap();
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        // Only manifests: skip Comtrya.yaml configs, files used by actions and known mismatches
        if path.extension().is_none_or(|ext| ext != "yaml")
            || path.file_name().is_some_and(|name| name == "Comtrya.yaml")
            || relative.components().any(|c| c.as_os_str() == "files")
            || KNOWN_MISMATCHES.contains(&relative_str.as_str())
        {
            continue;
        }

        let yaml = std::fs::read_to_string(path).unwrap();
        let manifest: serde_json::Value = serde_yaml_ng::from_str(&yaml)
            .unwrap_or_else(|err| panic!("{relative_str} is not valid YAML: {err}"));

        checked += 1;
        if !validator.is_valid(&manifest) {
            let errors: Vec<String> = validator
                .iter_errors(&manifest)
                .map(|e| format!("{} at {}", e, e.instance_path()))
                .take(3)
                .collect();
            failures.push(format!("{relative_str}: {}", errors.join("; ")));
        }
    }

    assert!(checked > 10, "only {checked} examples found");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn known_mismatches_still_fail() {
    let schema: serde_json::Value = serde_json::from_str(&generate_schema()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    // Once an example matches the schema, remove it from KNOWN_MISMATCHES
    for example in KNOWN_MISMATCHES {
        let yaml = std::fs::read_to_string(examples_dir().join(example)).unwrap();
        let manifest: serde_json::Value = serde_yaml_ng::from_str(&yaml).unwrap();

        assert!(
            !validator.is_valid(&manifest),
            "{example} matches the schema now, remove it from KNOWN_MISMATCHES"
        );
    }
}
