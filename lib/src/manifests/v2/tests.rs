//! Loading format 2 manifests

use super::super::{load_all, LoadErrorKind, Loaded};
use crate::contexts::Contexts;
use crate::values::Value;
use std::collections::BTreeMap;
use std::fs;

/// Contexts with `user.name` = `ada` and `os.name` = `linux`
fn contexts() -> Contexts {
    let one = |key: &str, value: &str| BTreeMap::from([(key.to_string(), Value::from(value))]);
    Contexts::from([
        (String::from("user"), one("name", "ada")),
        (String::from("os"), one("name", "linux")),
    ])
}

/// Loads a manifest directory with one file, `file` with `contents`
fn load(file: &str, contents: &str) -> Loaded {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(file), contents).unwrap();
    load_all(dir.path().to_path_buf(), &contexts())
}

/// The error of the one manifest that failed
fn error(file: &str, contents: &str) -> String {
    let loaded = load(file, contents);
    assert_eq!(1, loaded.errors.len(), "{:?}", loaded.manifests);
    loaded.errors[0].kind.to_string()
}

#[test]
fn a_version_2_manifest_loads() {
    let loaded = load(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    path: /tmp/x\n",
    );

    let manifest = &loaded.manifests["a"];
    assert_eq!(Some(2), manifest.version);
    assert_eq!("directory.create", manifest.actions[0].to_string());
}

#[test]
fn a_version_2_toml_manifest_loads() {
    let loaded = load(
        "a.toml",
        "version = 2\n[[actions]]\naction = \"directory.create\"\npath = \"/tmp/{{ user.name }}\"\n",
    );

    let crate::actions::Actions::DirectoryCreate(action) = &loaded.manifests["a"].actions[0] else {
        panic!("not directory.create");
    };
    assert_eq!("/tmp/ada", action.action.path);
}

#[test]
fn templates_are_rendered_in_values() {
    let loaded = load(
        "a.yaml",
        "version: 2\nactions:\n  - action: command.run\n    command: echo\n    args: [\"{{ user.name }}\", plain]\n",
    );

    let crate::actions::Actions::CommandRun(action) = &loaded.manifests["a"].actions[0] else {
        panic!("not command.run");
    };
    assert_eq!(vec!["ada", "plain"], action.action.args);
}

#[test]
fn a_false_where_skips_templates_that_would_fail() {
    let loaded = load(
        "a.yaml",
        "version: 2\nwhere: os.name == \"windows\"\ndepends: [b]\nactions:\n  - action: directory.create\n    path: \"{{ env.ONLY_ON_WINDOWS }}\"\n",
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    let manifest = &loaded.manifests["a"];
    assert_eq!(Some(r#"os.name == "windows""#), manifest.r#where.as_deref());
    assert_eq!(vec!["b"], manifest.depends);
    assert!(manifest.actions.is_empty());
}

#[test]
fn a_failing_template_names_the_value() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    path: a\n  - action: file.copy\n    from: a\n    to: \"{{ user.hom_dir }}\"\n",
    );

    assert!(error.contains("actions[1].to"), "{error}");
}

#[test]
fn where_conditions_are_not_templates() {
    let loaded = load(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    path: a\n    where: '\"{{\" != \"x\"'\n",
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
}

#[test]
fn an_action_alias_names_the_action() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: cmd.run\n    command: echo\n",
    );

    assert!(
        error.contains("actions[0]: `cmd.run` is another name for `command.run`"),
        "{error}"
    );
}

#[test]
fn a_field_alias_names_the_field() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: file.copy\n    source: a\n    to: b\n",
    );

    assert!(
        error.contains("actions[0]: `source` is another name for `from`"),
        "{error}"
    );
}

#[test]
fn file_link_knows_only_from_and_to() {
    let source = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: file.link\n    source: a\n    to: b\n",
    );
    let target = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: file.link\n    from: a\n    target: b\n",
    );

    assert!(
        source.contains("actions[0]: `source` is another name for `from`"),
        "{source}"
    );
    assert!(
        target.contains("actions[0]: `target` is another name for `to`"),
        "{target}"
    );
}

#[test]
fn file_link_with_from_and_to_loads() {
    let loaded = load(
        "a.yaml",
        "version: 2\nactions:\n  - action: file.link\n    from: a\n    to: b\n",
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
}

#[test]
fn version_1_file_link_keeps_source_and_target() {
    let loaded = load(
        "a.yaml",
        "actions:\n  - action: file.link\n    source: a\n    target: b\n  - action: file.link\n    from: a\n    to: b\n",
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert_eq!(2, loaded.manifests["a"].actions.len());
}

#[test]
fn a_field_alias_in_a_variant_names_the_variant() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: command.run\n    command: echo\n    variants:\n      - where: \"true\"\n        command: echo\n        sudo: true\n",
    );

    assert!(
        error.contains("actions[0]: variant 1: `sudo` is another name for `privileged`"),
        "{error}"
    );
}

#[test]
fn a_title_replaces_the_summary() {
    let loaded = load(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    title: Make the cache\n    path: a\n  - action: directory.create\n    path: b\n",
    );

    let actions = &loaded.manifests["a"].actions;
    assert_eq!("Make the cache", actions[0].summarize());
    assert_eq!("Create directory b", actions[1].summarize());
}

#[test]
fn a_title_that_is_not_text_is_an_error() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    title: [a]\n    path: a\n",
    );

    assert!(
        error.contains("actions[0]: `title` has to be text"),
        "{error}"
    );
}

#[test]
fn a_missing_field_names_the_action() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: directory.create\n    path: a\n  - action: file.copy\n    to: b\n",
    );

    assert!(
        error.contains("actions[1]: missing field `from`"),
        "{error}"
    );
}

#[test]
fn a_misspelled_field_is_hinted() {
    let error = error(
        "a.yaml",
        "version: 2\nactions:\n  - action: file.copy\n    form: a\n    to: b\n",
    );

    assert!(error.contains("did you mean `from`?"), "{error}");
}

#[test]
fn version_1_still_has_its_aliases_and_no_title() {
    let loaded = load(
        "a.yaml",
        "actions:\n  - action: cmd.run\n    command: echo\n    title: x\n",
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    assert_eq!("Run echo", loaded.manifests["a"].actions[0].summarize());
}

#[test]
fn a_templated_version_2_is_an_error() {
    let error = error("a.yaml", "version: {{ 1 + 1 }}\nactions: []\n");

    assert!(error.contains("`version: 2` has to be readable"), "{error}");
}

#[test]
fn a_newer_version_is_refused() {
    let loaded = load("a.yaml", "version: 3\nactions: []\n");

    assert!(matches!(
        loaded.errors[0].kind,
        LoadErrorKind::NewerVersion {
            found: 3,
            supported: 2
        }
    ));
}
