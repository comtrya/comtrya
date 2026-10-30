//! `file.*` actions: copy, download, link, change the owner of, remove and unpack files, and
//! what they share

pub mod chown;
pub mod copy;
pub mod download;
pub mod link;
pub mod remove;
pub mod unarchive;

use crate::actions::{Action, Error, Result};
use crate::atoms;
use crate::atoms::directory::Create as DirCreate;
use crate::manifests::Manifest;
use crate::steps::Step;
use normpath::PathExt;
use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Deserializer};
use std::path::{Path, PathBuf};

/// Actions that read files from the manifest's `files` directory
pub trait FileAction: Action {
    /// `path` below the manifest's `files` directory, which must exist
    fn resolve(&self, manifest: &Manifest, path: &str) -> Result<PathBuf> {
        let path = files_dir(manifest, path)?.join(path);

        path.normalize()
            .map(|path| path.into_path_buf())
            .map_err(|_| Error::NotFound(path))
    }

    /// The contents of `path` below the manifest's `files` directory
    fn load(&self, manifest: &Manifest, path: &str) -> Result<Vec<u8>> {
        let path = files_dir(manifest, path)?.join(path);

        std::fs::read(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => Error::NotFound(path),
            _ => atoms::Error::io("read", &path)(err).into(),
        })
    }
}

/// The manifest's `files` directory, to find `path` in
fn files_dir(manifest: &Manifest, path: &str) -> Result<PathBuf> {
    manifest
        .root_dir
        .as_ref()
        .map(|root_dir| root_dir.join("files"))
        .ok_or_else(|| Error::NoManifestDir(path.to_string()))
}

/// Creates the directory that `path` will be written to
fn create_parent(path: &Path) -> Result<Step> {
    let parent = path.parent().ok_or_else(|| Error::Invalid {
        field: "to",
        problem: format!("{} has no parent directory", path.display()),
    })?;

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
fn from_octal<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
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
