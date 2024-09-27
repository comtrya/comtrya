use crate::contexts::privilege::Privilege;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, vec};
use tracing::{instrument, trace, warn};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub manifest_paths: Vec<String>,

    #[serde(default)]
    pub variables: BTreeMap<String, String>,

    #[serde(default)]
    pub include_variables: Option<Vec<String>>,

    #[serde(default)]
    pub disable_update_check: bool,

    #[serde(default)]
    pub privilege: Privilege,
}

/// Check the current working directory for a `Comtrya.yaml` file
/// If that doesn't exist, we'll check the platforms config directory
/// for comtrya/Comtrya.yaml
#[instrument(name = "load_config", level = "info")]
pub fn load_config() -> Result<Config> {
    let config = match find_configs() {
        Some(config_path) => {
            let yaml = std::fs::read_to_string(&config_path)
                .with_context(|| "Found Comtrya.yaml, but was unable to read the contents.")?;

            let mut config = match yaml.trim().is_empty() {
                true => Config {
                    ..Default::default()
                },

                false => serde_yml::from_str(yaml.as_str())
                    .with_context(|| "Found Comtrya.yaml, but couldn't deserialize the YAML.")?,
            };

            // The existence of the config file allows an implicit manifests location of .
            if config.manifest_paths.is_empty() {
                if let Some(parent) = config_path.parent() {
                    config.manifest_paths.push(parent.display().to_string());
                }
            }

            config
        }

        None => Config {
            manifest_paths: vec![String::from(".")],
            ..Default::default()
        },
    };

    Ok(config)
}

fn find_configs() -> Option<PathBuf> {
    // Check current working directory first
    if let Ok(cwd) = std::env::current_dir() {
        let local_config = cwd.join("Comtrya.yaml");

        if local_config.is_file() {
            warn!("Comtrya.yaml found in current working directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in current working directory");
    }

    // Check platform's config dir
    if let Some(config_dir) = dirs_next::config_dir() {
        let local_config = config_dir.join("Comtrya.yaml");

        if local_config.is_file() {
            warn!("Comtrya.yaml found in users config directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in users config directory");
    };

    None
}
