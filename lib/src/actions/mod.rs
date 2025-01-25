mod binary;
pub mod command;
mod directory;
mod file;
mod git;
mod group;
mod macos;
mod package;
mod user;

use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{contexts::Contexts, utilities::password_manager::PasswordManager};
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
use macos::MacOSDefault;
use package::{PackageInstall, PackageRepository};
use rhai::Engine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tracing::{debug, error, info, instrument, warn};
use user::add::UserAdd;

use self::user::add_group::UserAddGroup;

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ConditionalVariantAction<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "where")]
    pub condition: Option<String>,

    #[serde(default)]
    pub variants: Vec<Variant<T>>,
}

#[derive(JsonSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "where")]
    pub condition: Option<String>,
}

impl<T> Action for ConditionalVariantAction<T>
where
    T: Action,
{
    fn summarize(&self) -> String {
        self.action.summarize()
    }

    fn plan(&self, manifest: &Manifest, context: &Contexts) -> Result<Vec<Step>, anyhow::Error> {
        let engine = Engine::new();
        let mut scope = crate::contexts::to_rhai(context);

        let variant = self.variants.iter().find(|variant| {
            if variant.condition.is_none() {
                return false;
            }

            // .unwrap() is safe here because we checked for None above
            let condition = variant.condition.clone().unwrap();
            match engine.eval_with_scope::<bool>(&mut scope, condition.as_str()) {
                Ok(b) => b,
                Err(error) => {
                    error!("Failed execution condition for action: {}", error);
                    false
                }
            }
        });

        if let Some(variant) = variant {
            return variant.action.plan(manifest, context);
        }

        if self.condition.is_none() {
            return self.action.plan(manifest, context);
        }

        // .unwrap() is safe here because we checked for None above
        let condition = self.condition.as_ref().unwrap();

        match engine.eval_with_scope::<bool>(&mut scope, condition.as_str()) {
            Ok(true) => self.action.plan(manifest, context),
            Ok(false) => Ok(vec![]),
            Err(error) => Err(anyhow!("Failed execution condition for action: {}", error)),
        }
    }
}

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "action")]
pub enum Actions {
    #[serde(rename = "command.run", alias = "cmd.run")]
    CommandRun(ConditionalVariantAction<RunCommand>),

    #[serde(rename = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(ConditionalVariantAction<DirectoryCopy>),

    #[serde(rename = "directory.create", alias = "dir.create")]
    DirectoryCreate(ConditionalVariantAction<DirectoryCreate>),

    #[serde(rename = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    #[serde(rename = "file.chown")]
    FileChown(ConditionalVariantAction<FileChown>),

    #[serde(rename = "file.download")]
    FileDownload(ConditionalVariantAction<FileDownload>),

    #[serde(rename = "file.link")]
    FileLink(ConditionalVariantAction<FileLink>),

    #[serde(rename = "file.remove")]
    FileRemove(ConditionalVariantAction<FileRemove>),

    #[serde(rename = "file.unarchive")]
    FileUnarchive(ConditionalVariantAction<FileUnarchive>),

    #[serde(rename = "directory.remove", alias = "dir.remove")]
    DirectoryRemove(ConditionalVariantAction<DirectoryRemove>),

    #[serde(
        rename = "binary.github",
        alias = "binary.gh",
        alias = "bin.github",
        alias = "bin.gh"
    )]
    BinaryGitHub(ConditionalVariantAction<BinaryGitHub>),

    #[serde(rename = "git.clone")]
    GitClone(ConditionalVariantAction<GitClone>),

    #[serde(rename = "group.add")]
    GroupAdd(ConditionalVariantAction<GroupAdd>),

    #[serde(rename = "macos.default")]
    MacOSDefault(ConditionalVariantAction<MacOSDefault>),

    #[serde(rename = "package.install", alias = "package.installed")]
    PackageInstall(ConditionalVariantAction<PackageInstall>),

    #[serde(rename = "package.repository", alias = "package.repo")]
    PackageRepository(ConditionalVariantAction<PackageRepository>),

    #[serde(rename = "user.add")]
    UserAdd(ConditionalVariantAction<UserAdd>),

    #[serde(rename = "user.group")]
    UserAddGroup(ConditionalVariantAction<UserAddGroup>),
}

impl Actions {
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
            Actions::BinaryGitHub(_) => "github.binary",
            Actions::GitClone(_) => "git.clone",
            Actions::GroupAdd(_) => "group.add",
            Actions::MacOSDefault(_) => "macos.default",
            Actions::PackageInstall(_) => "package.install",
            Actions::PackageRepository(_) => "package.repository",
            Actions::UserAdd(_) => "user.add",
            Actions::UserAddGroup(_) => "user.group",
        };

        write!(f, "{}", name)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// Output / response
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionError {
    /// Error message
    pub message: String,
}

impl<E: std::error::Error> From<E> for ActionError {
    fn from(e: E) -> Self {
        ActionError {
            message: format!("{}", e),
        }
    }
}

#[async_trait::async_trait]
pub trait Action: Send + Sync {
    fn summarize(&self) -> String {
        warn!("need to define action summarize");
        "not found action summarize".to_string()
    }
    fn plan(&self, manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>>;

    #[instrument(skip_all)]
    async fn execute(
        &self,
        dry_run: bool,
        action: &Actions,
        manifest: &Manifest,
        contexts: &Contexts,
        password_manager: Option<PasswordManager>,
    ) -> anyhow::Result<()> {
        let steps: Vec<_> = match self.plan(manifest, contexts) {
            Ok(steps) => steps
                .into_iter()
                .filter(|step| step.do_initializers_allow_us_to_run())
                .filter(|step| match step.atom.plan() {
                    Ok(outcome) => outcome.should_run,
                    Err(_) => false,
                })
                .collect(),
            Err(err) => {
                error!("Failed Processing: {action}. Action failed to get plan: {err:?}",);
                return Ok(());
            }
        };

        if steps.is_empty() {
            info!("nothing to be done to reconcile action");
            return Ok(());
        }

        if dry_run {
            return Ok(());
        }

        for mut step in steps {
            if let Err(e) = step.execute(password_manager.clone()).await {
                debug!("{e}");
            }
        }
        info!("{}", self.summarize());
        Ok(())
    }
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
        let m: Manifest = serde_yml::from_str(content).unwrap();

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
}
