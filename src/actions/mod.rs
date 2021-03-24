mod command;
mod directory;
mod file;
mod package;

use crate::manifests::Manifest;
use command::run::CommandRun;
use directory::copy::DirectoryCopy;
use file::copy::FileCopy;
use file::link::FileLink;
use package::install::PackageInstall;
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(alias = "command.run", alias = "cmd.run")]
    CommandRun(CommandRun),
    #[serde(alias = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(DirectoryCopy),
    #[serde(alias = "file.copy")]
    FileCopy(FileCopy),
    #[serde(alias = "file.link")]
    FileLink(FileLink),
    #[serde(alias = "package.install", alias = "package.installed")]
    PackageInstall(PackageInstall),
}

impl Actions {
    pub fn inner_ref(&self) -> &dyn Action {
        match self {
            Actions::CommandRun(a) => a,
            Actions::DirectoryCopy(a) => a,
            Actions::FileCopy(a) => a,
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

pub trait ActionResultExt<M, T> {
    fn context(self, message: M) -> Result<T, ActionError>;
}

impl<M: std::fmt::Display, T, E: std::error::Error> ActionResultExt<M, T> for Result<T, E> {
    fn context(self, message: M) -> Result<T, ActionError> {
        self.map_err(|e| ActionError {
            message: format!("{} because of {}", message, e.to_string()),
        })
    }
}

pub trait Action {
    fn run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult, ActionError>;

    fn dry_run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult, ActionError>;
}
