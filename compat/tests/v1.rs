//! Loads manifests with this checkout's lib and with the last release, and compares what they
//! mean: which manifests exist, their names, `where`, dependencies and every action with its
//! variants. A refactor that changes any of this for an existing manifest fails here.
//!
//! When a difference is intended, put it in the release notes and record it in
//! `known_differences` instead of changing the fixtures.

use pretty_assertions::assert_eq;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Fake facts for the systems manifests usually distinguish
fn facts() -> Vec<(&'static str, Value)> {
    let user = |name: &str, id: &str, home: &str| {
        json!({
            "id": id,
            "name": name,
            "username": name,
            "home_dir": home,
            "config_dir": format!("{home}/.config"),
            "data_dir": format!("{home}/.local/share"),
            "data_local_dir": format!("{home}/.local/share"),
            "document_dir": format!("{home}/Documents"),
        })
    };
    let os = |name: &str, family: &str, distribution: &str| {
        json!({
            "name": name,
            "family": family,
            "distribution": distribution,
            "codename": "unknown",
            "version": "1.0",
            "bitness": "64-bit",
            "edition": "unknown",
            "hostname": "compat",
        })
    };
    let variables = json!({ "app": "comtrya", "fkbr": "sxoe", "kuci": "xhpv" });
    let facts = |user: Value, os: Value, provider: Option<&str>| {
        let mut facts = json!({
            "user": user,
            "os": os,
            "env": {},
            "variables": variables,
            "privilege": { "privilege": "sudo" },
            "packages": {},
        });
        if let Some(provider) = provider {
            facts["packages"]["provider"] = json!(provider);
        }
        facts
    };

    vec![
        (
            "debian",
            facts(
                user("ada", "1000", "/home/ada"),
                os("linux", "unix", "Debian"),
                Some("aptitude"),
            ),
        ),
        (
            "arch as root",
            facts(
                user("root", "0", "/root"),
                os("linux", "unix", "Arch Linux"),
                Some("pacman"),
            ),
        ),
        (
            "macos",
            facts(
                user("ada", "501", "/Users/ada"),
                os("macos", "unix", "Mac OS"),
                Some("homebrew"),
            ),
        ),
        (
            "windows",
            facts(
                user("ada", "1001", "C:/Users/ada"),
                os("windows", "windows", "Windows"),
                Some("winget"),
            ),
        ),
        (
            "unknown os",
            facts(
                user("ada", "1000", "/home/ada"),
                os("plan9", "unknown", "unknown"),
                None,
            ),
        ),
    ]
}

/// Directories of manifests to compare
fn manifest_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    vec![root.join("fixtures/v1"), root.join("../examples")]
}

/// Intended differences to the release, as (manifest directory, manifest name)
fn known_differences() -> Vec<(&'static str, &'static str)> {
    vec![
        // 0.11.0 accepts `repository` and `reference` again, 0.10.0 refuses them
        ("v1", "git-clone-reference"),
    ]
}

/// Drops `null` fields, so a new optional field doesn't count as a difference.
/// A field whose value turns into `null` still shows up as missing
fn without_nulls(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .filter(|(_, value)| !value.is_null())
                .map(|(key, value)| (key, without_nulls(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_nulls).collect()),
        value => value,
    }
}

fn current(dir: &Path, facts: &Value) -> BTreeMap<String, Value> {
    let contexts: comtrya_lib::contexts::Contexts =
        serde_json::from_value(facts.clone()).expect("facts are valid contexts");
    comtrya_lib::manifests::load(dir.to_path_buf(), &contexts)
        .into_iter()
        .map(|(name, manifest)| (name, without_nulls(serde_json::to_value(manifest).unwrap())))
        .collect()
}

fn released(dir: &Path, facts: &Value) -> BTreeMap<String, Value> {
    let contexts: released::contexts::Contexts =
        serde_json::from_value(facts.clone()).expect("facts are valid contexts");
    released::manifests::load(dir.to_path_buf(), &contexts)
        .into_iter()
        .map(|(name, manifest)| (name, without_nulls(serde_json::to_value(manifest).unwrap())))
        .collect()
}

#[test]
fn manifests_mean_what_they_meant_in_the_last_release() {
    for dir in manifest_dirs() {
        let dir = dir.canonicalize().unwrap();
        let dir_name = dir.file_name().unwrap().to_string_lossy().to_string();

        for (system, facts) in facts() {
            let mut current = current(&dir, &facts);
            let mut released = released(&dir, &facts);

            for (known_dir, name) in known_differences() {
                if known_dir == dir_name {
                    current.remove(name);
                    released.remove(name);
                }
            }

            assert_eq!(
                released.keys().collect::<Vec<_>>(),
                current.keys().collect::<Vec<_>>(),
                "manifest names in {dir_name} on {system}"
            );

            for (name, manifest) in &released {
                assert_eq!(
                    manifest, &current[name],
                    "manifest {name} in {dir_name} on {system}"
                );
            }
        }
    }
}

/// Guards the fixtures themselves: they only protect what they still contain
#[test]
fn fixtures_cover_the_v1_rules() {
    let dir = manifest_dirs()[0].canonicalize().unwrap();
    let (_, debian) = &facts()[0];
    let manifests = current(&dir, debian);
    let names: Vec<&str> = manifests.keys().map(String::as_str).collect();

    // main.yaml takes its directory's name, .toml stays in the name, depth 3 still loads,
    // files/ is skipped, a broken template and a newer version drop the manifest, and a
    // false `where` keeps it without rendering
    assert_eq!(
        vec![
            "aliases",
            "deep.a.b.deep",
            "git-clone-reference",
            "nested",
            "nested.other",
            "templates",
            "toml.command.toml",
            "variants",
            "versioned",
            "where-false-template",
        ],
        names
    );

    let aliases = manifests["aliases"]["actions"].as_array().unwrap();
    let actions: Vec<&str> = aliases
        .iter()
        .map(|action| action["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        vec![
            "command.run",
            "directory.copy",
            "directory.create",
            "directory.remove",
            "file.copy",
            "file.download",
            "file.unarchive",
            "binary.github",
            "package.install",
            "package.repository",
            "package.install",
        ],
        actions
    );

    let clone = &manifests["git-clone-reference"]["actions"][0];
    assert_eq!("https://github.com/comtrya/comtrya", clone["repo_url"]);
    assert_eq!("v0.9.2", clone["reference"]);
}
