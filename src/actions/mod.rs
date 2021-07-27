mod command;
mod directory;
mod file;
mod package;

use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use command::run::RunCommand;
use directory::{DirectoryCopy, DirectoryCreate};
use file::copy::FileCopy;
use file::download::FileDownload;
use file::link::FileLink;
use package::install::PackageInstall;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ConditionalVariantAction<T> {
    #[serde(flatten)]
    pub action: T,

    #[serde(rename = "only")]
    pub condition: Option<String>,

    #[serde(default)]
    pub variants: Vec<Variant<T>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    fn plan(&self, manifest: &Manifest, context: &Contexts) -> Vec<Step> {
        let variant = self.variants.iter().find(|variant| {
            if variant.condition.is_none() {
                return false;
            }

            let condition = variant.condition.clone().unwrap();

            let mut expr = eval::Expr::new(condition);

            for (key, value) in context {
                expr = expr.value(key, value)
            }

            match expr.exec().unwrap_or(eval::to_value(false)) {
                eval::Value::Bool(true) => true,
                _ => false,
            }
        });

        if variant.is_some() {
            return variant.unwrap().action.plan(manifest, context);
        }

        if self.condition.is_none() {
            return self.action.plan(manifest, context);
        }

        let condition = self.condition.clone().unwrap();

        let mut expr = eval::Expr::new(condition);

        for (key, value) in context {
            expr = expr.value(key, value)
        }

        match expr.exec().unwrap_or(eval::to_value(false)) {
            eval::Value::Bool(true) => self.action.plan(manifest, context),
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(alias = "command.run", alias = "cmd.run")]
    CommandRun(ConditionalVariantAction<RunCommand>),

    #[serde(alias = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(ConditionalVariantAction<DirectoryCopy>),

    #[serde(alias = "directory.create", alias = "dir.create")]
    DirectoryCreate(ConditionalVariantAction<DirectoryCreate>),

    #[serde(alias = "file.copy")]
    FileCopy(ConditionalVariantAction<FileCopy>),

    #[serde(alias = "file.download")]
    FileDownload(ConditionalVariantAction<FileDownload>),

    #[serde(alias = "file.link")]
    FileLink(ConditionalVariantAction<FileLink>),

    #[serde(alias = "package.install", alias = "package.installed")]
    PackageInstall(ConditionalVariantAction<PackageInstall>),
}

impl Actions {
    pub fn inner_ref(&self) -> &dyn Action {
        match self {
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::DirectoryCreate(a) => a,
            Actions::FileCopy(a) => a,
            Actions::FileDownload(a) => a,
            Actions::FileLink(a) => a,
            Actions::PackageInstall(a) => a,
        }
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

pub trait Action {
    fn plan(&self, manifest: &Manifest, context: &Contexts) -> Vec<Step>;
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
  only: user.username != "root"
  command: echo
  args:
    - hi
  variants:
    - where: Debian
      command: halt
"#;
        let m: Manifest = serde_yaml::from_str(content).unwrap();

        let action = &m.actions[0];

        let ext = match action {
            Actions::CommandRun(cr) => cr,
            _ => panic!("did not get a command to run"),
        };
        assert_eq!(ext.condition, Some("user.username != \"root\"".into()));
        assert_eq!(
            ext.action,
            RunCommand {
                command: "echo".into(),
                args: vec!["hi".into()],
                sudo: false,
                dir: None,
            }
        );

        let variant = &ext.variants[0];
        assert_eq!(variant.condition, Some(String::from("Debian")));
        assert_eq!(variant.action.command, "halt");
    }
}
