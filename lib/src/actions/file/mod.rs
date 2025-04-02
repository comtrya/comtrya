pub mod chown;
pub mod copy;
pub mod download;
pub mod link;
pub mod remove;
pub mod unarchive;

use crate::actions::Action;
use crate::manifests::Manifest;
use anyhow::{anyhow, Context, Result};
use normpath::PathExt;
use serde::{de::Error, Deserialize, Deserializer};
use std::path::PathBuf;

pub trait FileAction: Action {
    fn resolve(&self, manifest: &Manifest, path: &str) -> anyhow::Result<PathBuf> {
        Ok(manifest
            .root_dir
            .as_ref()
            .context("Failed because manifest has no root_dir")?
            .join("files")
            .join(path)
            .normalize()
            .with_context(|| {
                format!(
                    "Resolution of {} failed in manifest {}",
                    path,
                    manifest.get_name()
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

        std::fs::read(&*file_path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => anyhow!(
                "Failed because {} was not found",
                file_path.to_string_lossy()
            ),
            _ => anyhow!("Failed because {}", e.to_string()),
        })
    }
}

fn from_octal<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let chmod = String::deserialize(deserializer)?;
    u32::from_str_radix(&chmod, 8).map_err(D::Error::custom)
}

fn default_chmod() -> u32 {
    0o644
}
