mod command;
mod directory;
mod file;
mod package;

use crate::manifests::Manifest;
use crate::steps::Step;
use command::run::RunCommand;
use directory::{DirectoryCopy, DirectoryCreate};
use file::copy::FileCopy;
use file::download::FileDownload;
use file::link::FileLink;
use package::install::PackageInstall;
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(alias = "command.run", alias = "cmd.run")]
    CommandRun(RunCommand),

    #[serde(alias = "directory.copy", alias = "dir.copy")]
    DirectoryCopy(DirectoryCopy),

    #[serde(alias = "directory.create", alias = "dir.create")]
    DirectoryCreate(DirectoryCreate),

    #[serde(alias = "file.copy")]
    FileCopy(FileCopy),

    #[serde(alias = "file.download")]
    FileDownload(FileDownload),

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
    fn plan(&self, manifest: &Manifest, context: &Context) -> Vec<Step>;
}
