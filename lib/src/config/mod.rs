//! `Comtrya.yaml`: the configuration of a run

use crate::actions::PackageProviders;
use crate::contexts::privilege::Privilege;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The contents of `Comtrya.yaml`. Everything is optional
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    /// Where the manifests are: directories, or git repositories as `https://`, `git://` or
    /// `ssh://` URLs
    #[serde(default)]
    pub manifest_paths: Vec<String>,

    /// Values for manifests, the `variables` context
    #[serde(default)]
    pub variables: BTreeMap<String, String>,

    /// More variables, from `file+yaml://`, `file+toml://`, `dns+txt://` or `op://` sources, the
    /// `include_variables` context
    #[serde(default)]
    pub include_variables: Option<Vec<String>>,

    /// Don't look for a newer comtrya at startup
    #[serde(default)]
    pub disable_update_check: bool,

    /// The program that runs privileged commands, `sudo` by default
    #[serde(default)]
    pub privilege: Privilege,

    /// Used by `package.*` actions without a `provider`, instead of the default for the OS
    #[serde(default)]
    pub package_provider: Option<PackageProviders>,
}
