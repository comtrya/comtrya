pub mod copy;
pub mod download;
pub mod link;

use crate::actions::Action;
use crate::manifests::Manifest;
use anyhow::{anyhow, Result};
use normpath::PathExt;
use serde::{de::Error, Deserialize, Deserializer};
use std::path::PathBuf;

pub trait FileAction: Action {
    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf, anyhow::Error> {
        Ok(manifest
            .root_dir
            .clone()
            .unwrap()
            .join("files")
            .join(path)
            .normalize()
            .map_err(|e| {
                anyhow!(
                    "Resolution of {} failed in manifest {} because {}",
                    path.to_string(),
                    manifest.name.as_ref().unwrap(),
                    e.to_string()
                )
            })?
            .as_path()
            .to_path_buf())
    }

    fn load(&self, manifest: &Manifest, path: &str) -> Result<Vec<u8>> {
        use std::io::ErrorKind;
        let file_path = manifest.root_dir.clone().unwrap().join("files").join(path);

        std::fs::read(file_path.clone()).map_err(|e| match e.kind() {
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
