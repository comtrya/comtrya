//! Variants that override fields of their action

use super::super::super::{load_all, Loaded};
use crate::actions::Actions;
use crate::contexts::Contexts;
use crate::values::Value;
use std::collections::BTreeMap;
use std::fs;

/// Contexts with `os.name` = `linux`
fn linux() -> Contexts {
    Contexts::from([(
        String::from("os"),
        BTreeMap::from([(String::from("name"), Value::from("linux"))]),
    )])
}

/// Loads a manifest directory with one file, `file` with `contents`
fn load(file: &str, contents: &str) -> Loaded {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(file), contents).unwrap();
    load_all(dir.path().to_path_buf(), &linux())
}

/// The error of the one manifest that failed
fn error(contents: &str) -> String {
    let loaded = load("a.yaml", contents);
    assert_eq!(1, loaded.errors.len(), "{:?}", loaded.manifests);
    loaded.errors[0].kind.to_string()
}

/// `from`, `to` and `chmod` of the `file.copy` action that `contents` holds, as the variant
/// for linux picks it
fn copy_on_linux(file: &str, contents: &str) -> (String, String, u32) {
    let loaded = load(file, contents);
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    let Actions::FileCopy(action) = &loaded.manifests["a"].actions[0] else {
        panic!("not file.copy");
    };
    let copy = action.select(&linux()).unwrap().unwrap();
    (copy.from.clone(), copy.to.clone(), copy.chmod)
}

const OVERRIDE_ONE_FIELD: &str = r#"
version: 2
actions:
  - action: file.copy
    from: zshrc
    to: /home/ada/.zshrc
    chmod: 600
    variants:
      - where: os.name == "linux"
        to: /home/ada/.config/zsh/.zshrc
"#;

#[test]
fn a_variant_overrides_only_the_fields_it_sets() {
    let (from, to, chmod) = copy_on_linux("a.yaml", OVERRIDE_ONE_FIELD);

    assert_eq!("zshrc", from);
    assert_eq!("/home/ada/.config/zsh/.zshrc", to);
    assert_eq!(0o600, chmod);
}

#[test]
fn a_toml_variant_overrides_only_the_fields_it_sets() {
    let toml = r#"
version = 2
[[actions]]
action = "file.copy"
from = "zshrc"
to = "/home/ada/.zshrc"

[[actions.variants]]
where = 'os.name == "linux"'
to = "/home/ada/.config/zsh/.zshrc"
"#;
    let (from, to, _) = copy_on_linux("a.toml", toml);

    assert_eq!("zshrc", from);
    assert_eq!("/home/ada/.config/zsh/.zshrc", to);
}

#[test]
fn a_list_in_a_variant_replaces_the_whole_list() {
    let loaded = load(
        "a.yaml",
        r#"
version: 2
actions:
  - action: command.run
    command: echo
    args: [a, b]
    variants:
      - where: os.name == "linux"
        args: [c]
"#,
    );
    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
    let Actions::CommandRun(action) = &loaded.manifests["a"].actions[0] else {
        panic!("not command.run");
    };

    let run = action.select(&linux()).unwrap().unwrap();
    assert_eq!("echo", run.command);
    assert_eq!(vec!["c"], run.args);
    assert_eq!(
        vec!["a", "b"],
        action.action.args,
        "the action itself is unchanged"
    );
}

#[test]
fn the_action_applies_when_no_variant_matches() {
    let (_, to, _) = copy_on_linux(
        "a.yaml",
        &OVERRIDE_ONE_FIELD.replace(r#"os.name == "linux""#, r#"os.name == "macos""#),
    );

    assert_eq!("/home/ada/.zshrc", to);
}

#[test]
fn a_variant_does_not_take_the_where_of_its_action() {
    let loaded = load(
        "a.yaml",
        r#"
version: 2
actions:
  - action: directory.create
    path: /a
    where: os.name == "macos"
    variants:
      - where: os.name == "linux"
        path: /b
"#,
    );
    let Actions::DirectoryCreate(action) = &loaded.manifests["a"].actions[0] else {
        panic!("not directory.create");
    };

    assert_eq!("/b", action.select(&linux()).unwrap().unwrap().path);
}

#[test]
fn an_unknown_key_in_a_variant_is_not_an_error() {
    let loaded = load(
        "a.yaml",
        &OVERRIDE_ONE_FIELD.replace("        to: /home", "        too: x\n        to: /home"),
    );

    assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
}

#[test]
fn an_alias_in_a_variant_is_an_error() {
    let error = error(&OVERRIDE_ONE_FIELD.replace("        to: /home", "        target: /home"));

    assert!(
        error.contains("`target` is another name for `to`"),
        "{error}"
    );
}

#[test]
fn keys_of_the_whole_action_are_errors_in_a_variant() {
    for key in [
        "action: file.copy",
        "title: x",
        "continue_on_error: true",
        "variants: []",
    ] {
        let error = error(&OVERRIDE_ONE_FIELD.replace(
            "        to: /home",
            &format!("        {key}\n        to: /home"),
        ));

        let name = key.split(':').next().unwrap();
        assert!(error.contains("actions[0].variants[0]"), "{key}: {error}");
        assert!(error.contains(&format!("`{name}`")), "{key}: {error}");
    }
}

#[test]
fn a_variant_that_is_not_a_map_is_an_error() {
    let error = error(
        "version: 2\nactions:\n  - action: directory.create\n    path: /a\n    variants: [x]\n",
    );

    assert!(error.contains("actions[0].variants[0]"), "{error}");
}

#[test]
fn version_1_variants_are_still_whole_actions() {
    let error = error(
        r#"
actions:
  - action: file.copy
    from: a
    to: b
    variants:
      - where: "true"
        to: c
"#,
    );

    assert!(error.contains("variant 1: missing field `from`"), "{error}");
}
