mod command;
mod directory;
mod file;
mod package;

use crate::manifests::Manifest;
use anyhow::Result;
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

pub trait Action {
    fn run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult>;

    fn dry_run(&self, manifest: &Manifest, context: &Context) -> Result<ActionResult>;

    fn changeset(&self, manifest: &Manifest, _context: &Context) -> Option<ChangeSet> {
        Some(ChangeSet {
            changes: vec![Change {
                action: manifest.name.clone().unwrap_or("unknown".to_string()),
                change: String::from("No ChangeSet implementation, assuming always needs executed"),
            }],
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryChange {}

#[derive(Debug)]
pub struct Change {
    pub action: String, // Which action: "package.install"
    pub change: String, // What needs to happen: "The requested packages, vim and emacs, are currently missing"
}

#[derive(Debug)]
pub struct ChangeSet {
    changes: Vec<Change>,
}
