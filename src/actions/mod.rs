use crate::manifests::Manifest;
use package::install::PackageInstall;
use serde::{Deserialize, Serialize};

pub mod command;
pub mod package;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Actions {
    #[serde(alias = "package.install")]
    PackageInstall(PackageInstall),
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
    fn run(self: &Self, manifest: &Manifest) -> Result<ActionResult, ActionError>;
}
