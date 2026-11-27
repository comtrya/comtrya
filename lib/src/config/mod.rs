//! `Comtrya.yaml`: the configuration of a run. It can also be `Comtrya.yml` or `Comtrya.toml`

use crate::actions::PackageProviders;
use crate::contexts::privilege::Privilege;
use crate::manifests::{Ignore, InvalidGlob};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error as ThisError;

/// The names the config file can have, in the order they're looked for in a directory
pub const FILE_NAMES: [&str; 3] = ["Comtrya.yaml", "Comtrya.yml", "Comtrya.toml"];

/// Why the config file can't be read
#[derive(Debug, ThisError)]
pub enum ParseError {
    /// It isn't valid YAML, or not a config
    #[error(transparent)]
    Yaml(#[from] serde_yaml_ng::Error),
    /// It isn't valid TOML, or not a config
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// A glob in `ignore` can't be read
    #[error(transparent)]
    Ignore(#[from] InvalidGlob),
}

/// Reads the contents of the config file at `path`: TOML for a `.toml` file, YAML otherwise. An
/// empty file is the default config
///
/// # Errors
///
/// When the contents aren't valid TOML or YAML for a config, or a glob in `ignore` can't be read
pub fn parse(contents: &str, path: &Path) -> Result<Config, ParseError> {
    if contents.trim().is_empty() {
        return Ok(Config::default());
    }

    let config: Config = match path.extension().and_then(|extension| extension.to_str()) {
        Some("toml") => toml::from_str(contents)?,
        _ => serde_yaml_ng::from_str(contents)?,
    };
    config.ignored()?;
    Ok(config)
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

    /// Globs of paths below the manifest directory that aren't manifests, like app configs
    /// written in YAML. Relative to the manifest directory, see [`Ignore`]
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl Config {
    /// The compiled `ignore` globs
    ///
    /// # Errors
    ///
    /// When a glob can't be read
    pub fn ignored(&self) -> Result<Ignore, InvalidGlob> {
        Ignore::new(&self.ignore)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_is_read_from_yaml_and_toml() {
        let yaml = parse("ignore: [\"zsh/**\"]\n", Path::new("Comtrya.yaml")).unwrap();
        let toml = parse("ignore = [\"zsh/**\"]\n", Path::new("Comtrya.toml")).unwrap();

        assert_eq!(vec!["zsh/**"], yaml.ignore);
        assert_eq!(vec!["zsh/**"], toml.ignore);
    }

    #[test]
    fn an_invalid_ignore_glob_fails_the_config() {
        let err = parse("ignore: [\"zsh/[a\"]\n", Path::new("Comtrya.yaml")).unwrap_err();

        assert!(matches!(err, ParseError::Ignore(_)), "{err}");
    }
}
