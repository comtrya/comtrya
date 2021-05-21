mod command;
mod directory;
mod file;
mod package;

use std::fmt::Display;

use crate::{
    atoms::{finalizers, initializers, Atom},
    manifests::Manifest,
};
use command::run::RunCommand;
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
    CommandRun(RunCommand),
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

pub struct ActionAtom {
    pub atom: Box<dyn Atom>,
    pub initializers: Vec<initializers::FlowControl>,
    pub finalizers: Vec<finalizers::FlowControl>,
}

impl Display for ActionAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActionAtom: {} (Not printing initializers and finalizers yet)",
            self.atom
        )
    }
}

pub trait Action {
    fn plan(&self, manifest: &Manifest, context: &Context) -> Vec<ActionAtom>;
}
