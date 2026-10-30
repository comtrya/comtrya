//! Brings a clone up to date with its remote

use super::super::Change;
use super::clone_repository;
use crate::atoms::Check;
use gix::Url;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Replaces the clone in `directory` with a fresh, shallow clone of the remote's current state.
/// The old clone is kept if anything fails, so an unreachable remote costs nothing.
///
/// A fresh clone is used instead of a fetch and checkout in the old one because the clone
/// is a cache: local changes are meant to be discarded, and replacing the directory also
/// removes files that are gone from the remote, which a checkout in gix would not.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Update {
    /// The URL of the repository
    #[serde(with = "super::url_as_string")]
    pub repository: Url,
    /// The existing clone
    pub directory: PathBuf,
    /// Branch or tag to check out, the remote's `HEAD` if `None`
    pub reference: Option<String>,
}

impl std::fmt::Display for Update {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GitUpdate {} in {}",
            self.repository,
            self.directory.display()
        )
    }
}

/// A path next to `directory`, named like it with `suffix` appended
fn sibling(directory: &Path, suffix: &str) -> PathBuf {
    let mut name = directory.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

/// Removes `path` if it exists
fn remove_dir_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
        _ => Ok(()),
    }
}

impl Update {
    /// Where the old clone is kept while it is replaced
    pub(crate) fn replaced_dir(directory: &Path) -> PathBuf {
        sibling(directory, ".replaced")
    }

    /// Puts the clone in `fresh` in place of the one in `directory`, restoring the old one
    /// if that fails
    fn replace_with(&self, fresh: &Path) -> anyhow::Result<()> {
        let old = Self::replaced_dir(&self.directory);
        remove_dir_if_exists(&old)?;
        std::fs::rename(&self.directory, &old)?;
        if let Err(err) = std::fs::rename(fresh, &self.directory) {
            std::fs::rename(&old, &self.directory)?;
            return Err(err.into());
        }

        // The update is done, a leftover old clone is only waste
        let _ = remove_dir_if_exists(&old);
        Ok(())
    }
}

impl Change for Update {
    #[instrument(name = "git.update.plan", level = "info", skip(self))]
    fn check(&self) -> anyhow::Result<Check> {
        Ok(Check::drift_if(self.directory.exists()))
    }

    #[instrument(name = "git.update.execute", level = "info", skip(self))]
    fn execute(&self) -> anyhow::Result<()> {
        let fresh = sibling(&self.directory, ".updating");
        remove_dir_if_exists(&fresh)?;

        let result = clone_repository(&self.repository, &fresh, self.reference.as_deref(), true)
            .and_then(|()| self.replace_with(&fresh));
        if result.is_err() {
            let _ = remove_dir_if_exists(&fresh);
        }
        result
    }
}
