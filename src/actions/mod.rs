mod command;
mod file;
mod package;

use crate::manifest::Manifest;
use file::copy::FileCopy;
use package::install::PackageInstall;
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(alias = "package.install", alias = "package.installed")]
    PackageInstall(PackageInstall),

    #[serde(alias = "file.copy")]
    FileCopy(FileCopy),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionResult {
    /// Output / response
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionError {
    /// Error message
    message: String,
}

pub trait Action {
    fn run(
        self: &Self,
        manifest: &Manifest,
        context: &Context,
    ) -> Result<ActionResult, ActionError>;
}
