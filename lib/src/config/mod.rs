//! `Comtrya.yaml`: the configuration of a run. It can also be `Comtrya.yml` or `Comtrya.toml`

use crate::actions::PackageProviders;
use crate::contexts::privilege::Privilege;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The names the config file can have, in the order they're looked for in a directory
pub const FILE_NAMES: [&str; 3] = ["Comtrya.yaml", "Comtrya.yml", "Comtrya.toml"];

/// Reads the contents of the config file at `path`: TOML for a `.toml` file, YAML otherwise. An
/// empty file is the default config
pub fn parse(contents: &str, path: &Path) -> anyhow::Result<Config> {
    if contents.trim().is_empty() {
        return Ok(Config::default());
    }

    match path.extension().and_then(|extension| extension.to_str()) {
        Some("toml") => Ok(toml::from_str(contents)?),
        _ => Ok(serde_yaml_ng::from_str(contents)?),
    }
}

/// The contents of `Comtrya.yaml`. Everything is optional
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    /// Where the manifests are: directories, or git repositories as `https://`, `git://` or
    /// `ssh://` URLs
    #[serde(default)]
    pub manifest_paths: Vec<String>,

    /// The manifests `comtrya apply` runs when `-m` doesn't name any, with their dependencies.
    /// All manifests when it's empty
    #[serde(default)]
    pub default_manifests: Vec<String>,

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
