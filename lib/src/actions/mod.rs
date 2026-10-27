//! Actions: what manifests ask for, like `file.copy` or `package.install`. Each action plans the
//! steps that bring the system there

mod binary;
mod command;
mod directory;
mod file;
mod git;
mod group;
mod macos;
mod package;
pub use package::PackageProviders;
mod user;

use crate::actions::macos::MacOSDefault;
use crate::{
    contexts::{evaluate_condition, Contexts},
    manifests::Manifest,
    steps::Step,
};
use anyhow::anyhow;
use binary::BinaryGitHub;
use command::run::RunCommand;
use directory::{DirectoryCopy, DirectoryCreate, DirectoryRemove};
use file::chown::FileChown;
use file::copy::FileCopy;
use file::download::FileDownload;
use file::link::FileLink;
use file::remove::FileRemove;
use file::unarchive::FileUnarchive;
use git::GitClone;
use group::add::GroupAdd;
use package::{PackageInstall, PackageRepository};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::ops::Deref;
use tracing::{error, warn};
use user::add::UserAdd;

use self::user::add_group::UserAddGroup;

/// An action with its `where` condition and its variants, see [`ConditionalVariantAction::select`]
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConditionalVariantAction<T> {
    /// The action's own fields
    #[serde(flatten)]
    pub action: T,

    /// `where`: the action only runs when it's true
    #[serde(rename = "where")]
    pub condition: Option<String>,

    /// Other versions of the action, for other conditions
    #[serde(default)]
    pub variants: Vec<Variant<T>>,
}

/// Another version of an action, used instead of it when its `where` is true
#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant<T> {
    /// The fields of this version
    #[serde(flatten)]
    pub action: T,

    /// `where`: when this version applies. A variant without one never applies
    #[serde(rename = "where")]
    pub condition: Option<String>,
}

impl<T> Variant<T> {
    /// A variant of `action` for `condition`
    pub fn new(action: T, condition: Option<String>) -> Self {
        Self { action, condition }
    }
}

impl<T> Action for ConditionalVariantAction<T>
where
    T: Action,
{
    fn summarize(&self) -> String {
        self.action.summarize()
    }

    fn plan(&self, manifest: &Manifest, context: &Contexts) -> Result<Vec<Step>, anyhow::Error> {
        match self.select(context)? {
            Some(action) => action.plan(manifest, context),
            None => Ok(vec![]),
        }
    }
}

impl<T> ConditionalVariantAction<T> {
    /// The action that applies: the first variant whose `where` is true, else the action itself
    /// if its own `where` is true or missing, else `None`. These are the v1 rules:
    ///
    /// - A variant applies even when the action's own `where` is false
    /// - A variant without `where` never applies
    /// - A variant's `where` that can't be evaluated counts as false, the action's is an error
    pub fn select(&self, contexts: &Contexts) -> anyhow::Result<Option<&T>> {
        let variant = self.variants.iter().find(|variant| {
            variant.condition.as_ref().is_some_and(|condition| {
                evaluate_condition(condition, contexts).unwrap_or_else(|error| {
                    error!("Failed execution condition for action: {}", error);
                    false
                })
            })
        });

        if let Some(variant) = variant {
            return Ok(Some(&variant.action));
        }

        let Some(condition) = &self.condition else {
            return Ok(Some(&self.action));
        };

        match evaluate_condition(condition, contexts) {
            Ok(true) => Ok(Some(&self.action)),
            Ok(false) => Ok(None),
            Err(error) => Err(anyhow!("Failed execution condition for action: {error}")),
        }
    }
}

/// Every action, by its name in manifests
#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    /// `command.run`, or `cmd.run`
    #[serde(rename = "command.run", alias = "cmd.run")]
    CommandRun(ConditionalVariantAction<RunCommand>),

    /// `directory.copy`, or `dir.copy`
    #[serde(rename = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(ConditionalVariantAction<DirectoryCopy>),

    /// `directory.create`, or `dir.create`
    #[serde(rename = "directory.create", alias = "dir.create")]
    DirectoryCreate(ConditionalVariantAction<DirectoryCreate>),

    /// `file.copy`
    #[serde(rename = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    /// `file.chown`
    #[serde(rename = "file.chown")]
    FileChown(ConditionalVariantAction<FileChown>),

    /// `file.download`
    #[serde(rename = "file.download")]
    FileDownload(ConditionalVariantAction<FileDownload>),

    /// `file.link`
    #[serde(rename = "file.link")]
    FileLink(ConditionalVariantAction<FileLink>),

    /// `file.remove`
    #[serde(rename = "file.remove")]
    FileRemove(ConditionalVariantAction<FileRemove>),

    /// `file.unarchive`
    #[serde(rename = "file.unarchive")]
    FileUnarchive(ConditionalVariantAction<FileUnarchive>),

    /// `directory.remove`, or `dir.remove`
    #[serde(rename = "directory.remove", alias = "dir.remove")]
    DirectoryRemove(ConditionalVariantAction<DirectoryRemove>),

    /// `binary.github`, or `binary.gh`, `bin.github`, `bin.gh`
    #[serde(
        rename = "binary.github",
        alias = "binary.gh",
        alias = "bin.github",
        alias = "bin.gh"
    )]
    BinaryGitHub(ConditionalVariantAction<BinaryGitHub>),

    /// `git.clone`
    #[serde(rename = "git.clone")]
    GitClone(ConditionalVariantAction<GitClone>),

    /// `group.add`
    #[serde(rename = "group.add")]
    GroupAdd(ConditionalVariantAction<GroupAdd>),

    /// `macos.default`
    #[serde(rename = "macos.default")]
    MacOSDefault(ConditionalVariantAction<MacOSDefault>),

    /// `package.install`, or `package.installed`
    #[serde(rename = "package.install", alias = "package.installed")]
    PackageInstall(ConditionalVariantAction<PackageInstall>),

    /// `package.repository`, or `package.repo`
    #[serde(rename = "package.repository", alias = "package.repo")]
    PackageRepository(ConditionalVariantAction<PackageRepository>),

    /// `user.add`
    #[serde(rename = "user.add")]
    UserAdd(ConditionalVariantAction<UserAdd>),

    /// `user.group`
    #[serde(rename = "user.group")]
    UserAddGroup(ConditionalVariantAction<UserAddGroup>),
}

impl Actions {
    /// The action, whichever it is
    pub fn inner_ref(&self) -> &dyn Action {
        match self {
            Actions::BinaryGitHub(a) => a,
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::DirectoryCreate(a) => a,
            Actions::FileCopy(a) => a,
            Actions::FileChown(a) => a,
            Actions::FileDownload(a) => a,
            Actions::FileLink(a) => a,
            Actions::FileUnarchive(a) => a,
            Actions::GitClone(a) => a,
            Actions::GroupAdd(a) => a,
            Actions::MacOSDefault(a) => a,
            Actions::PackageInstall(a) => a,
            Actions::PackageRepository(a) => a,
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::FileRemove(a) => a,
            Actions::DirectoryRemove(a) => a,
        }
    }
}

impl Deref for Actions {
    type Target = dyn Action;
    fn deref(&self) -> &Self::Target {
        match self {
            Actions::BinaryGitHub(a) => a,
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::DirectoryCreate(a) => a,
            Actions::FileCopy(a) => a,
            Actions::FileChown(a) => a,
            Actions::FileDownload(a) => a,
            Actions::FileLink(a) => a,
            Actions::FileUnarchive(a) => a,
            Actions::GitClone(a) => a,
            Actions::GroupAdd(a) => a,
            Actions::MacOSDefault(a) => a,
            Actions::PackageInstall(a) => a,
            Actions::PackageRepository(a) => a,
            Actions::UserAdd(a) => a,
            Actions::UserAddGroup(a) => a,
            Actions::FileRemove(a) => a,
            Actions::DirectoryRemove(a) => a,
        }
    }
}

impl Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Actions::CommandRun(_) => "command.run",
            Actions::DirectoryCopy(_) => "directory.copy",
            Actions::DirectoryCreate(_) => "directory.create",
            Actions::FileCopy(_) => "file.copy",
            Actions::FileChown(_) => "file.chown",
            Actions::FileDownload(_) => "file.download",
            Actions::FileLink(_) => "file.link",
            Actions::FileRemove(_) => "file.remove",
            Actions::FileUnarchive(_) => "file.unarchive",
            Actions::DirectoryRemove(_) => "directory.remove",
            Actions::BinaryGitHub(_) => "binary.github",
            Actions::GitClone(_) => "git.clone",
            Actions::GroupAdd(_) => "group.add",
            Actions::MacOSDefault(_) => "macos.default",
            Actions::PackageInstall(_) => "package.install",
            Actions::PackageRepository(_) => "package.repository",
            Actions::UserAdd(_) => "user.add",
            Actions::UserAddGroup(_) => "user.group",
        };

        write!(f, "{name}")
    }
}

/// Not used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// Output / response
    pub message: String,
}

/// Not used
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionError {
    /// Error message
    pub message: String,
}

impl<E: std::error::Error> From<E> for ActionError {
    fn from(e: E) -> Self {
        ActionError {
            message: format!("{e}"),
        }
    }
}

/// What every action can do
pub trait Action {
    /// One line about what the action does, for the output
    fn summarize(&self) -> String {
        warn!("need to define action summarize");
        "not found action summarize".to_string()
    }
    /// The steps that bring the system to what the action asks for. Doesn't change the system
    fn plan(&self, manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>>;
}

#[cfg(test)]
mod tests {
    use crate::actions::{command::run::RunCommand, Actions};
    use crate::manifests::Manifest;

    #[test]
    fn can_parse_some_advanced_stuff() {
        let content = r#"
actions:
- action: command.run
  command: echo
  args:
    - hi
  variants:
    - where: Debian
      command: halt
"#;
        let m: Manifest = serde_yaml_ng::from_str(content).unwrap();

        let action = &m.actions[0];

        let ext = match action {
            Actions::CommandRun(cr) => cr,
            _ => panic!("did not get a command to run"),
        };

        assert_eq!(
            ext.action,
            RunCommand {
                command: "echo".into(),
                args: vec!["hi".into()],
                privileged: false,
                dir: std::env::current_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
                ..Default::default()
            }
        );

        let variant = &ext.variants[0];
        assert_eq!(variant.condition, Some(String::from("Debian")));
        assert_eq!(variant.action.command, "halt");
    }

    #[test]
    fn display_shows_the_action_name() {
        let content = r#"
actions:
- action: bin.gh
  name: comtrya
  directory: /tmp
  repository: comtrya/comtrya
"#;
        let m: Manifest = serde_yaml_ng::from_str(content).unwrap();
        let action = &m.actions[0];

        let serialized = serde_json::to_value(action).unwrap();
        assert_eq!(serialized["action"], action.to_string());
        assert_eq!("binary.github", action.to_string());
    }

    mod select {
        use super::super::{ConditionalVariantAction, Variant};
        use crate::contexts::Contexts;
        use crate::values::Value;
        use pretty_assertions::assert_eq;
        use std::collections::BTreeMap;

        fn linux() -> Contexts {
            Contexts::from([(
                String::from("os"),
                BTreeMap::from([(String::from("name"), Value::from("linux"))]),
            )])
        }

        fn action(
            condition: Option<&str>,
            variants: &[(Option<&str>, &'static str)],
        ) -> ConditionalVariantAction<&'static str> {
            ConditionalVariantAction {
                action: "action",
                condition: condition.map(String::from),
                variants: variants
                    .iter()
                    .map(|(condition, name)| Variant::new(*name, condition.map(String::from)))
                    .collect(),
            }
        }

        fn selected(action: &ConditionalVariantAction<&'static str>) -> Option<&'static str> {
            action.select(&linux()).unwrap().copied()
        }

        const LINUX: Option<&str> = Some(r#"os.name == "linux""#);
        const MACOS: Option<&str> = Some(r#"os.name == "macos""#);

        #[test]
        fn without_conditions_the_action_applies() {
            assert_eq!(Some("action"), selected(&action(None, &[])));
        }

        #[test]
        fn the_action_where_decides_without_a_matching_variant() {
            assert_eq!(
                Some("action"),
                selected(&action(LINUX, &[(MACOS, "macos")]))
            );
            assert_eq!(None, selected(&action(MACOS, &[(MACOS, "macos")])));
        }

        #[test]
        fn the_first_matching_variant_wins() {
            let action = action(
                None,
                &[(MACOS, "macos"), (LINUX, "first"), (LINUX, "second")],
            );

            assert_eq!(Some("first"), selected(&action));
        }

        #[test]
        fn a_variant_applies_even_when_the_action_where_is_false() {
            assert_eq!(Some("linux"), selected(&action(MACOS, &[(LINUX, "linux")])));
        }

        #[test]
        fn a_variant_without_where_never_applies() {
            assert_eq!(
                Some("action"),
                selected(&action(None, &[(None, "variant")]))
            );
        }

        #[test]
        fn a_broken_variant_where_counts_as_false() {
            let action = action(None, &[(Some("os.name =="), "broken"), (LINUX, "linux")]);

            assert_eq!(Some("linux"), selected(&action));
        }

        #[test]
        fn a_broken_action_where_is_an_error() {
            assert!(action(Some("os.name =="), &[]).select(&linux()).is_err());
        }
    }
}
