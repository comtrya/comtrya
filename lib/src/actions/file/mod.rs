//! `file.*` actions: copy, download, link, change the owner of, remove and unpack files, and
//! what they share

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

pub mod chown;
pub mod copy;
pub mod download;
pub mod link;
pub mod remove;
pub mod unarchive;

use crate::actions::Action;
use crate::atoms::directory::Create as DirCreate;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::{anyhow, Result};
use normpath::PathExt;
use schemars::JsonSchema;
use serde::{de::Error, Deserialize, Deserializer};
use std::path::{Path, PathBuf};

/// Actions that read files from the manifest's `files` directory
pub trait FileAction: Action {
    /// `path` below the manifest's `files` directory, which must exist
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

    /// The contents of `path` below the manifest's `files` directory
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

/// Creates the directory that `path` will be written to
fn create_parent(path: &Path) -> Result<Step> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent directory of path: {}", path.display()))?;

    Ok(Step::new(DirCreate {
        path: parent.into(),
    }))
}

/// Gives `path` to `user` and `group`, if both are set. Ownership is ignored on Windows
#[cfg(unix)]
fn change_owner(path: &Path, user: &Option<String>, group: &Option<String>) -> Option<Step> {
    use crate::atoms::file::Chown;

    Some(Step::new(Chown {
        path: path.into(),
        owner: user.clone()?,
        group: group.clone()?,
    }))
}

#[cfg(not(unix))]
fn change_owner(_: &Path, _: &Option<String>, _: &Option<String>) -> Option<Step> {
    None
}

/// File permissions in octal notation, written as a number (`644`) or a string (`"0644"`)
#[derive(JsonSchema, Deserialize)]
#[serde(untagged)]
pub(crate) enum OctalMode {
    /// Like `644`, read as octal digits
    Number(u32),
    /// Like `"0644"`
    Text(String),
}

/// Reads `chmod` as octal, whether it's written as a number or a string
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

/// `rw-r--r--`
fn default_chmod() -> u32 {
    0o644
}

#[cfg(all(test, unix))]
mod tests {
    use super::change_owner;
    use std::path::Path;

    #[test]
    fn the_owner_needs_a_user_and_a_group() {
        let user = Some(String::from("ferris"));
        let group = Some(String::from("crabs"));
        let path = Path::new("a");

        assert!(change_owner(path, &user, &group).is_some());
        assert!(change_owner(path, &user, &None).is_none());
        assert!(change_owner(path, &None, &group).is_none());
    }
}
