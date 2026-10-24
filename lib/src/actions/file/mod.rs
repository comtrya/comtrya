pub mod chown;
pub mod copy;
pub mod download;
pub mod link;
pub mod remove;
pub mod unarchive;

use crate::actions::Action;
use crate::manifests::Manifest;
use anyhow::{anyhow, Result};
use normpath::PathExt;
use schemars::JsonSchema;
use serde::{de::Error, Deserialize, Deserializer};
use std::path::PathBuf;

pub trait FileAction: Action {
    fn resolve(&self, manifest: &Manifest, path: &str) -> anyhow::Result<PathBuf> {
        Ok(manifest
            .root_dir
            .clone()
            .ok_or_else(|| anyhow!("Failed because manifest has no root_dir"))?
            .join("files")
            .join(path)
            .normalize()
            .map_err(|e| {
                anyhow!(
                    "Resolution of {} failed in manifest {} because {}",
                    path,
                    manifest
                        .name
                        .as_ref()
                        .unwrap_or(&"cannot extract manifest name".to_string()),
                    e
                )
            })?
            .as_path()
            .to_path_buf())
    }

    fn load(&self, manifest: &Manifest, path: &str) -> Result<Vec<u8>> {
        use std::io::ErrorKind;
        let file_path = manifest
            .root_dir
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Cannot extract root dir"))?
            .join("files")
            .join(path);

        std::fs::read(file_path.clone()).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!(
                "Failed because {} was not found",
                file_path.to_string_lossy()
            ),
            _ => anyhow!("Failed because {e}"),
        })
    }
}

/// File permissions in octal notation, written as a number (`644`) or a string (`"0644"`)
#[derive(JsonSchema, Deserialize)]
#[serde(untagged)]
pub(crate) enum OctalMode {
    Number(u32),
    Text(String),
}

fn from_octal<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    // The digits of a number are read as octal as well, so `644` means `0o644`
    let chmod = match OctalMode::deserialize(deserializer)? {
        OctalMode::Number(number) => number.to_string(),
        OctalMode::Text(text) => text,
    };

    u32::from_str_radix(&chmod, 8).map_err(D::Error::custom)
}

fn default_chmod() -> u32 {
    0o644
}
