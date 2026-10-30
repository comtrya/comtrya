//! Unpacks `.tar.gz` archives

use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::atoms::{Change, Check, Error, Result};

/// Unpacks a `.tar.gz` archive into a directory
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unarchive {
    /// The archive
    pub origin: PathBuf,
    /// The directory to unpack into
    pub dest: PathBuf,
    /// Unpack again if `dest` exists already
    pub force: bool,
}

impl Change for Unarchive {
    // Determine if this atom needs to run
    fn check(&self) -> Result<Check> {
        if self.dest.exists() && !self.force {
            return Ok(Check::InSync);
        }

        Ok(Check::needs(&self.origin).unwrap_or(Check::Drift))
    }

    // Apply new to old
    fn execute(&self) -> Result<()> {
        let tar_gz = File::open(&self.origin).map_err(Error::io("open", &self.origin))?;
        let mut archive = Archive::new(GzDecoder::new(tar_gz));
        archive
            .unpack(&self.dest)
            .map_err(Error::io("unpack into", &self.dest))?;
        Ok(())
    }
}

impl std::fmt::Display for Unarchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let origin_path = self.origin.display().to_string();
        let dest_path = self.dest.display().to_string();

        write!(
            f,
            "The archive {origin_path} to be decompressed to {dest_path}"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn a_missing_archive_is_pending() {
        let dir = tempfile::tempdir().unwrap();
        let atom = Unarchive {
            origin: dir.path().join("archive.tar.gz"),
            dest: dir.path().join("unpacked"),
            force: false,
        };

        assert_eq!(
            Check::Pending {
                needs: dir.path().join("archive.tar.gz")
            },
            atom.check().unwrap()
        );
    }

    #[test]
    fn an_existing_destination_is_kept_unless_forced() {
        let dir = tempfile::tempdir().unwrap();
        let origin = dir.path().join("archive.tar.gz");
        std::fs::write(&origin, "").unwrap();
        let atom = |force| Unarchive {
            origin: origin.clone(),
            dest: dir.path().to_path_buf(),
            force,
        };

        assert_eq!(Check::InSync, atom(false).check().unwrap());
        assert_eq!(Check::Drift, atom(true).check().unwrap());
    }
}
