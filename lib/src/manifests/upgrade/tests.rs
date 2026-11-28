use super::*;
use crate::contexts::Contexts;
use crate::utilities::error_chain;
use pretty_assertions::assert_eq;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Contexts with a few `variables`, like the ones of the compat fixtures
fn contexts() -> Contexts {
    let variables = BTreeMap::from([
        (String::from("home"), String::from("/home/ada").into()),
        (String::from("mode"), String::from("0755").into()),
        (String::from("root"), String::from("true").into()),
        (String::from("app"), String::from("comtrya").into()),
        (String::from("fkbr"), String::from("sxoe").into()),
        (String::from("kuci"), String::from("xhpv").into()),
    ]);
    Contexts::from([(String::from("variables"), variables)])
}

/// The upgraded YAML manifest `text`
fn upgraded(text: &str) -> Upgraded {
    upgrade_text(text, Path::new("/dots/m.yaml"), &contexts()).unwrap()
}

/// Why the YAML manifest `text` can't be upgraded
fn refused(text: &str) -> String {
    let err = upgrade_text(text, Path::new("/dots/m.yaml"), &contexts()).unwrap_err();
    error_chain(&err)
}

#[test]
fn version_2_comes_first() {
    let text = "depends: [base]\nactions:\n  - action: command.run\n    command: echo\n";

    assert_eq!(
        "version: 2\ndepends:\n- base\nactions:\n- action: command.run\n  command: echo\n",
        upgraded(text).text
    );
}

#[test]
fn version_1_becomes_2() {
    let text = "version: 1\nactions: []\n";

    assert_eq!("version: 2\nactions: []\n", upgraded(text).text);
}

#[test]
fn old_action_and_field_names_become_the_one_name() {
    let text = "actions:\n  - action: cmd.run\n    command: echo\n    sudo: true\n  - action: file.copy\n    source: a\n    target: /tmp/a\n";

    assert_eq!(
        "version: 2\nactions:\n- action: command.run\n  command: echo\n  privileged: true\n- action: file.copy\n  from: a\n  to: /tmp/a\n",
        upgraded(text).text
    );
}

#[test]
fn file_link_source_and_target_become_from_and_to() {
    let text = "actions:\n  - action: file.link\n    source: zshrc\n    target: /home/ada/.zshrc\n";

    assert_eq!(
        "version: 2\nactions:\n- action: file.link\n  from: zshrc\n  to: /home/ada/.zshrc\n",
        upgraded(text).text
    );
}

#[test]
fn file_link_source_wins_over_from_as_in_version_1() {
    let text = "actions:\n  - action: file.link\n    from: old\n    source: zshrc\n    to: /x\n";

    let upgraded = upgraded(text);

    assert_eq!(
        "version: 2\nactions:\n- action: file.link\n  from: zshrc\n  to: /x\n",
        upgraded.text
    );
    assert!(upgraded.warnings[0].contains("`source` and `from` are both set"));
}

#[test]
fn an_alias_next_to_its_field_is_refused() {
    let text = "actions:\n  - action: file.copy\n    from: a\n    source: b\n    to: /x\n";

    assert!(refused(text).contains("`source` and `from` are the same field"));
}

#[test]
fn variants_keep_only_what_differs() {
    let text = "actions:
  - action: package.install
    name: fd
    provider: aptitude
    variants:
      - where: os.name == \"macos\"
        name: fd
        provider: homebrew
";

    assert_eq!(
        "version: 2
actions:
- action: package.install
  name: fd
  provider: aptitude
  variants:
  - where: os.name == \"macos\"
    provider: homebrew
",
        upgraded(text).text
    );
}

#[test]
fn a_variant_that_leaves_out_a_field_of_its_action_is_refused() {
    let text = "actions:
  - action: command.run
    command: echo
    dir: /tmp
    variants:
      - where: os.name == \"macos\"
        command: ls
";

    let refused = refused(text);

    assert!(
        refused.contains("actions[0].variants[0].dir is")
            && refused.contains("but \"/tmp\" in version 2"),
        "{refused}"
    );
    assert!(
        refused.contains("set the field in the variant"),
        "{refused}"
    );
}

#[test]
fn dot_sources_point_to_the_same_files() {
    let text = "actions:
  - action: file.copy
    from: ./zshrc
    to: /a
  - action: file.copy
    from: ../shared/x
    to: /b
  - action: directory.copy
    from: ./
    to: /c
  - action: file.unarchive
    from: ./a.tar.gz
    to: /d
";

    assert_eq!(
        "version: 2
actions:
- action: file.copy
  from: zshrc
  to: /a
- action: file.copy
  from: ./shared/x
  to: /b
- action: directory.copy
  from: ./files
  to: /c
- action: file.unarchive
  from: a.tar.gz
  to: /d
",
        upgraded(text).text
    );
}

#[test]
fn an_unarchive_source_above_the_working_directory_is_refused() {
    let text = "actions:\n  - action: file.unarchive\n    from: ../a.tar.gz\n    to: /d\n";

    assert!(refused(text).contains("relative to where comtrya runs"));
}

#[test]
fn values_that_start_with_a_template_get_quotes() {
    let text = "actions:
  - action: directory.create
    path: {{ variables.home }}/bin
  - action: command.run
    command: echo
    args:
      - {{ variables.home }}
";

    assert_eq!(
        "version: 2
actions:
- action: directory.create
  path: '{{ variables.home }}/bin'
- action: command.run
  command: echo
  args:
  - '{{ variables.home }}'
",
        upgraded(text).text
    );
}

#[test]
fn a_template_that_changes_the_type_is_checked() {
    // Version 1 reads the rendered `0755` as a number, version 2 too, since `chmod` can't hold
    // text
    let text = "actions:\n  - action: file.copy\n    from: a\n    to: /a\n    chmod: {{ variables.mode }}\n";

    assert!(upgraded(text)
        .text
        .contains("chmod: '{{ variables.mode }}'"));
}

#[test]
fn templates_around_keys_are_refused() {
    let text = "actions:\n{% if true %}\n  - action: command.run\n    command: echo\n{% endif %}\n";

    assert!(refused(text).contains("isn't valid YAML or TOML before its templates are rendered"));
}

#[test]
fn a_where_with_a_template_is_refused() {
    let text = "where: user.name == \"{{ variables.app }}\"\nactions: []\n";

    assert!(refused(text).contains("where has a template"));
}

#[test]
fn version_2_is_refused() {
    assert!(refused("version: 2\nactions: []\n").contains("already uses manifest version 2"));
}

#[test]
fn a_false_where_is_kept_and_doesnt_hide_the_actions_from_the_check() {
    let text = "where: os.name == \"nowhere\"\nactions:\n  - action: cmd.run\n    command: echo\n";

    assert_eq!(
        "version: 2\nwhere: os.name == \"nowhere\"\nactions:\n- action: command.run\n  command: echo\n",
        upgraded(text).text
    );
}

#[test]
fn comments_are_warned_about() {
    let text = "# my manifest\nactions: []\n";

    assert_eq!(
        vec![String::from(
            "comments aren't kept; copy the ones you need from the original"
        )],
        upgraded(text).warnings
    );
}

#[test]
fn toml_is_upgraded_as_toml() {
    let text = "[[actions]]\naction = \"cmd.run\"\ncommand = \"echo\"\nsudo = true\n";

    let upgraded = upgrade_text(text, Path::new("/dots/m.toml"), &contexts()).unwrap();

    assert_eq!(
        "version = 2\n\n[[actions]]\naction = \"command.run\"\ncommand = \"echo\"\nprivileged = true\n",
        upgraded.text
    );
}

#[test]
fn toml_values_that_start_with_a_template_get_quotes() {
    let text = "[[actions]]\naction = \"directory.create\"\npath = \"/x\"\nprivileged = {{ variables.root }}\n";

    let upgraded = upgrade_text(text, Path::new("/dots/m.toml"), &contexts()).unwrap();

    assert!(
        upgraded
            .text
            .contains("privileged = \"{{ variables.root }}\""),
        "{}",
        upgraded.text
    );
}

/// The version 1 compat fixtures that version 1 doesn't load here, with why
const NOT_LOADED: [(&str, &str); 4] = [
    ("broken.yaml", "it's broken on purpose"),
    ("newer.yaml", "it uses a newer version"),
    (
        "continue-on-error.yaml",
        "it has a `continue_on_error` version 1 rejects",
    ),
    (
        "privileged.yaml",
        "a variant leaves out a required field, which version 1 rejects",
    ),
];

/// The version 1 compat fixtures that aren't upgraded, with a part of the reason
const REFUSED: [(&str, &str); 6] = [
    // Their variants leave out fields of their action, which version 2 would take
    ("commands.yaml", "variants[0].dir is"),
    ("files.yaml", "variants[0].from is null in version 1"),
    ("packages.yaml", "variants[1].list is [] in version 1"),
    ("system.yaml", "variants[0].username is \"\" in version 1"),
    // `{% if %}` around actions
    (
        "templates.yaml",
        "isn't valid YAML or TOML before its templates are rendered",
    ),
    // Its template only renders where its `where` is true, so it can't be checked here
    ("where-false-template.yaml", "it doesn't load here"),
];

#[test]
fn the_compat_fixtures_upgrade_to_the_same_meaning_or_are_refused() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../compat/fixtures/v1");
    let mut unexpected = vec![];
    for path in fixtures(&dir) {
        let relative = path.strip_prefix(&dir).unwrap().display().to_string();
        if NOT_LOADED.iter().any(|(name, _)| *name == relative) {
            continue;
        }
        let refusal = REFUSED
            .iter()
            .find(|(name, _)| *name == relative)
            .map(|(_, reason)| *reason);
        match (upgrade(&path, &contexts()), refusal) {
            (Ok(_), None) => {}
            (Err(err), Some(reason)) if error_chain(&err).contains(reason) => {}
            (Ok(_), Some(reason)) => {
                unexpected.push(format!("{relative}: upgraded, expected {reason}"));
            }
            (Err(err), _) => unexpected.push(format!("{relative}: {}", error_chain(&err))),
        }
    }

    assert_eq!(Vec::<String>::new(), unexpected);
}

/// The manifest files below `dir`, without `files/` and the manifests both versions reject
fn fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut paths = vec![];
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            if name != "files" && name != "rejected" {
                paths.extend(fixtures(&path));
            }
        } else if name.ends_with(".yaml") || name.ends_with(".yml") || name.ends_with(".toml") {
            paths.push(path);
        }
    }
    paths.sort();
    paths
}
