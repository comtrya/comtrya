//! `file+toml://` and `file+yaml://`: variables from local files

use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use toml::Value;

/// Reads the top-level keys of a TOML file
pub fn toml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = toml::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key.to_string(), value.to_string());
    }

    Ok(())
}

/// Reads the top-level keys of a YAML file
pub fn yaml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = serde_yaml_ng::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key.to_string(), value.to_string());
    }

    Ok(())
}
