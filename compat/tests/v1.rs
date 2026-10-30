//! Loads manifests with this checkout's lib and with the last release, and compares what they
//! mean: which manifests exist, their names, `where`, dependencies and every action with its
//! variants, and under each set of facts, which manifests run and which variant of each action
//! applies. A refactor that changes any of this for an existing manifest fails here.
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
        // 0.11.0 runs file and directory actions with `privileged` as root, 0.10.0 ignored it
        ("v1", "privileged"),
        // 0.11.0 drops `.toml` from manifest names like `.yaml`, 0.10.0 kept it
        ("v1", "toml.command.toml"),
        ("v1", "toml.command"),
        ("v1", "toml.everything.toml"),
        ("v1", "toml.everything"),
        ("examples", "toml-examples.command.command.toml"),
        ("examples", "toml-examples.command.command"),
        // 0.11.0 doesn't load `Comtrya.yaml` next to the manifests as a manifest, 0.10.0 did
        ("examples", "Comtrya"),
    ]
}

/// Drops `null` fields and empty objects, so a new optional field, or a removed one that was
/// always empty, doesn't count as a difference. A field whose value turns into `null` still
/// shows up as missing
fn without_nulls(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .filter(|(_, value)| !value.is_null() && !is_empty_object(value))
                .map(|(key, value)| (key, without_nulls(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(without_nulls).collect()),
        value => value,
    }
}

/// Whether `value` is `{}`
fn is_empty_object(value: &Value) -> bool {
    value.as_object().is_some_and(|fields| fields.is_empty())
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

/// Records which action the released `ConditionalVariantAction::plan` picks, so the release's own
/// code decides which variant wins
#[derive(Default, serde::Deserialize)]
#[serde(transparent)]
struct Recorder(Value);

thread_local! {
    static PLANNED: std::cell::RefCell<Option<Value>> = const { std::cell::RefCell::new(None) };
}

impl released::actions::Action for Recorder {
    fn plan(
        &self,
        _: &released::manifests::Manifest,
        _: &released::contexts::Contexts,
    ) -> anyhow::Result<Vec<released::steps::Step>> {
        PLANNED.with(|planned| *planned.borrow_mut() = Some(self.0.clone()));
        Ok(vec![])
    }
}

/// What a manifest does under the facts: whether it runs, and which action or variant of each
/// action applies. `None` for an action means it's skipped, `"error"` that its `where` is broken
fn selections(
    manifest: &Value,
    manifest_applies: impl Fn(&str) -> bool,
    select: impl Fn(&Value) -> Result<Option<Value>, ()>,
) -> Value {
    if let Some(condition) = manifest["where"].as_str() {
        if !manifest_applies(condition) {
            return json!("manifest skipped");
        }
    }

    manifest["actions"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|action| match select(action) {
            Ok(selected) => json!(selected),
            Err(()) => json!("error"),
        })
        .collect()
}

fn current_selections(manifest: &Value, facts: &Value) -> Value {
    let contexts: comtrya_lib::contexts::Contexts = serde_json::from_value(facts.clone()).unwrap();

    selections(
        manifest,
        |condition| {
            comtrya_lib::contexts::evaluate_condition(condition, &contexts).unwrap_or(false)
        },
        |action| {
            let action: comtrya_lib::actions::ConditionalVariantAction<Value> =
                serde_json::from_value(action.clone()).unwrap();
            action
                .select(&contexts)
                .map(|selected| selected.cloned())
                .map_err(|_| ())
        },
    )
}

fn released_selections(manifest: &Value, facts: &Value) -> Value {
    use released::actions::Action;

    let contexts: released::contexts::Contexts = serde_json::from_value(facts.clone()).unwrap();

    selections(
        manifest,
        // How 0.10.0 evaluated a manifest's `where`
        |condition| {
            rhai::Engine::new()
                .eval_with_scope::<bool>(&mut released::contexts::to_rhai(&contexts), condition)
                .unwrap_or(false)
        },
        |action| {
            let action: released::actions::ConditionalVariantAction<Recorder> =
                serde_json::from_value(action.clone()).unwrap();
            PLANNED.with(|planned| planned.borrow_mut().take());
            action
                .plan(&Default::default(), &contexts)
                .map(|_| PLANNED.with(|planned| planned.borrow_mut().take()))
                .map_err(|_| ())
        },
    )
}

#[test]
fn variants_win_like_in_the_last_release() {
    for dir in manifest_dirs() {
        let dir = dir.canonicalize().unwrap();
        let dir_name = dir.file_name().unwrap().to_string_lossy().to_string();

        for (system, facts) in facts() {
            let current = current(&dir, &facts);

            for (name, manifest) in released(&dir, &facts) {
                if known_differences().contains(&(dir_name.as_str(), name.as_str())) {
                    continue;
                }

                assert_eq!(
                    released_selections(&manifest, &facts),
                    current_selections(&current[&name], &facts),
                    "what applies in manifest {name} in {dir_name} on {system}"
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

    // main.yaml takes its directory's name, .toml is dropped from the name, depth 3 still loads,
    // files/ is skipped, a broken template, a newer version, an unknown action, a missing
    // field and a scalar of the wrong type (rejected/) drop the manifest, and a false `where`
    // keeps it without rendering
    assert_eq!(
        vec![
            "aliases",
            "commands",
            "deep.a.b.deep",
            "files",
            "git-clone-reference",
            "manifest-keys",
            "manifest-minimal",
            "manifest-where-false",
            "nested",
            "nested.other",
            "packages",
            "scalars",
            "system",
            "templates",
            "toml.command",
            "toml.everything",
            "variant-rules",
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

    // What applies, shortened to `args[0]` or the package list, to see that the rules decide
    let applies = |manifest: &str, system: usize| -> Vec<String> {
        let (_, facts) = &facts()[system];
        let selections = current_selections(&manifests[manifest], facts);
        let Some(selections) = selections.as_array() else {
            return vec![selections.as_str().unwrap().to_string()];
        };
        selections
            .iter()
            .map(|selected| match selected {
                Value::Null => String::from("skipped"),
                Value::String(marker) => marker.clone(),
                selected if selected["args"].is_array() => {
                    selected["args"][0].as_str().unwrap().to_string()
                }
                selected => selected["list"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|package| package.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(","),
            })
            .collect()
    };
    let (debian, arch_as_root, macos, windows) = (0, 1, 2, 3);
    assert_eq!(
        vec!["linux", "unix", "action", "error", "skipped"],
        applies("variant-rules", debian)
    );
    assert_eq!(
        vec!["skipped", "unix", "action", "error", "skipped"],
        applies("variant-rules", macos)
    );
    assert_eq!(vec!["linux", "git"], applies("variants", debian));
    // root fails the action's `where`, but the linux variant applies anyway
    assert_eq!(
        vec!["linux", "git,base-devel"],
        applies("variants", arch_as_root)
    );
    assert_eq!(vec!["manifest skipped"], applies("variants", windows));

    let clone = &manifests["git-clone-reference"]["actions"][0];
    assert_eq!("https://github.com/comtrya/comtrya", clone["repo_url"]);
    assert_eq!("v0.9.2", clone["reference"]);
}

/// Every `.yaml` fixture below `dir`, parsed as written, so aliases are still the spelling used
fn raw_yaml_fixtures(dir: &Path) -> Vec<serde_yaml_ng::Value> {
    let mut fixtures = vec![];
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            fixtures.extend(raw_yaml_fixtures(&path));
        } else if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
        {
            let text = std::fs::read_to_string(&path).unwrap();
            fixtures.extend(serde_yaml_ng::from_str(&text));
        }
    }
    fixtures
}

/// The keys of one action entry and of its variants, next to the action's spelling
fn keys_of_action(action: &serde_yaml_ng::Value, used: &mut BTreeMap<String, Vec<String>>) {
    let Some(name) = action["action"].as_str() else {
        return;
    };
    let entries =
        std::iter::once(action).chain(action["variants"].as_sequence().into_iter().flatten());
    for entry in entries {
        let keys = entry
            .as_mapping()
            .into_iter()
            .flat_map(|entry| entry.keys());
        used.entry(name.to_string())
            .or_default()
            .extend(keys.filter_map(|key| key.as_str().map(String::from)));
    }
}

/// Without a fixture for an action, its spellings or its fields, a refactor could change what
/// they mean unnoticed
#[test]
fn fixtures_use_every_action_name_alias_and_field() {
    let mut used = BTreeMap::new();
    for manifest in raw_yaml_fixtures(&manifest_dirs()[0]) {
        for action in manifest["actions"].as_sequence().into_iter().flatten() {
            keys_of_action(action, &mut used);
        }
    }

    for (name, aliases) in comtrya_lib::actions::Actions::NAMES {
        let spellings: Vec<&str> = std::iter::once(name)
            .chain(aliases.iter())
            .copied()
            .collect();
        for spelling in &spellings {
            assert!(used.contains_key(*spelling), "no fixture uses `{spelling}`");
        }

        let used_keys: Vec<&String> = spellings
            .iter()
            .flat_map(|spelling| &used[*spelling])
            .collect();
        for key in comtrya_lib::actions::Actions::known_keys(name).unwrap() {
            assert!(
                used_keys.contains(&&key),
                "no fixture of {name} uses its field `{key}`"
            );
        }
    }
}
