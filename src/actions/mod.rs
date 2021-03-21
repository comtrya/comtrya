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

pub trait Action {
    fn run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult, ActionError>;
}
