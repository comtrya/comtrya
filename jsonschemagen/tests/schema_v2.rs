//! Checks the version 2 schema, and that the version 2 examples of the docs load and match it

use comtrya_lib::contexts::Contexts;
use comtrya_lib::manifests::load_all;
use comtrya_lib::values::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the binary like the mdBook workflow does for schema-v2.json
fn generate_schema() -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_jsonschemagen"))
        .args(["--version", "2"])
        .output()
        .expect("jsonschemagen should run");
    assert!(output.status.success());

    serde_json::from_slice(&output.stdout).expect("stdout should only contain the schema")
}

/// The version 2 manifests that the docs include
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/examples/v2")
}

/// The examples' manifests, with their path below [`examples_dir`]
fn examples() -> Vec<(String, PathBuf)> {
    let dir = examples_dir();
    walkdir::WalkDir::new(&dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "yaml"))
        .map(|entry| {
            let relative = entry.path().strip_prefix(&dir).unwrap();
            let relative = relative.to_string_lossy().replace('\\', "/");
            (relative, entry.into_path())
        })
        .collect()
}

/// The contexts that the examples' templates read
fn contexts() -> Contexts {
    let values = |pairs: &[(&str, &str)]| -> BTreeMap<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), Value::from(*value)))
            .collect()
    };
    Contexts::from([
        (
            String::from("user"),
            values(&[("home_dir", "/home/ada"), ("name", "ada")]),
        ),
        (String::from("os"), values(&[("name", "linux")])),
        (
            String::from("variables"),
            values(&[("greeting", "Hello"), ("as_root", "false")]),
        ),
    ])
}

#[test]
fn the_schema_is_for_version_2() {
    let schema = generate_schema();

    assert_eq!("Manifest (version 2)", schema["title"]);
    assert_eq!(2, schema["properties"]["version"]["const"]);
    assert_eq!(serde_json::json!(["version"]), schema["required"]);
}

/// The fields of the action `name` and of its variants
fn fields_of(schema: &serde_json::Value, name: &str) -> Vec<serde_json::Value> {
    let defs = &schema["$defs"];
    let action = schema["$defs"]["Actions"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["properties"]["action"]["const"] == name)
        .unwrap();
    let def = &defs[action["$ref"]
        .as_str()
        .unwrap()
        .trim_start_matches("#/$defs/")];
    let variant = def["properties"]["variants"]["items"]["$ref"]
        .as_str()
        .unwrap();
    let variant = &defs[variant.trim_start_matches("#/$defs/")];
    vec![def["properties"].clone(), variant["properties"].clone()]
}

#[test]
fn the_schema_knows_one_name_per_action_and_field() {
    let schema = generate_schema();
    let text = schema.to_string();

    assert!(!text.contains("x-aliases"), "aliases left in the schema");
    assert!(
        !text.contains(", or `"),
        "action aliases left in a description"
    );
    for fields in fields_of(&schema, "file.link") {
        assert!(
            fields.get("source").is_none(),
            "`source` is `from` in version 2"
        );
        assert!(
            fields.get("target").is_none(),
            "`target` is `to` in version 2"
        );
        assert!(fields.get("from").is_some() && fields.get("to").is_some());
    }
}

#[test]
fn only_command_run_can_register() {
    let schema = generate_schema();
    let defs = &schema["$defs"];
    let registering: Vec<&str> = schema["$defs"]["Actions"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|action| {
            let def = action["$ref"]
                .as_str()
                .unwrap()
                .trim_start_matches("#/$defs/");
            defs[def]["properties"].get("register").is_some()
        })
        .map(|action| action["properties"]["action"]["const"].as_str().unwrap())
        .collect();

    assert_eq!(vec!["command.run"], registering);
}

#[test]
fn the_examples_load() {
    let loaded = load_all(examples_dir(), &contexts());

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert_eq!(examples().len(), loaded.manifests.len());
    for (name, manifest) in &loaded.manifests {
        assert_eq!(Some(2), manifest.version, "{name}");
    }
}

#[test]
fn the_examples_match_the_schema() {
    let validator = jsonschema::validator_for(&generate_schema()).expect("a valid JSON schema");

    let mut failures = vec![];
    for (relative, path) in examples() {
        let yaml = std::fs::read_to_string(path).unwrap();
        let manifest: serde_json::Value = serde_yaml_ng::from_str(&yaml).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&manifest)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .take(3)
            .collect();
        if !errors.is_empty() {
            failures.push(format!("{relative}: {}", errors.join("; ")));
        }
    }

    assert!(examples().len() >= 6, "examples missing");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn version_1_names_are_rejected() {
    let validator = jsonschema::validator_for(&generate_schema()).unwrap();
    let manifest = serde_json::json!({
        "version": 2,
        "actions": [{ "action": "file.link", "source": "a", "target": "b" }]
    });

    assert!(!validator.is_valid(&manifest));
}
