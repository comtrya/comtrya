//! Copies the contents of a directory into another one

use crate::atoms::{Check, Error, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use std::path::{Path, PathBuf};

/// Copies the contents of a directory into another one, recursively.
///
/// `to` is created if missing. Files that exist in both are overwritten, files only in `to` are
/// kept. Symlinks are recreated as symlinks on unix; elsewhere the file they point to is copied.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Copy {
    /// The directory whose contents are copied
    pub from: PathBuf,
    /// The directory they are copied into
    pub to: PathBuf,
}

impl std::fmt::Display for Copy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The contents of the directory {} need to be copied to {}",
            self.from.display(),
            self.to.display(),
        )
    }
}

impl Change for Copy {
    /// Always runs: comparing every file would cost about as much as copying them
    fn check(&self) -> Result<Check> {
        Ok(Check::Drift)
    }

    fn execute(&self) -> Result<()> {
        refuse_copy_into_itself(&self.from, &self.to)?;
        copy_contents(&self.from, &self.to)?;

        Ok(())
    }
}

/// Fails when `to` is `from` or inside it, like `cp`: copying would never end, or copy files
/// onto themselves
fn refuse_copy_into_itself(from: &Path, to: &Path) -> Result<()> {
    let from_canonical = from.canonicalize().map_err(Error::io("read", from))?;
    let to_canonical = canonicalize_missing(to).map_err(Error::io("read", to))?;

    if to_canonical.starts_with(&from_canonical) {
        return Err(Error::CopyIntoItself {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
        });
    }

    Ok(())
}

/// Like [`Path::canonicalize`], but `path` and its parents don't have to exist: the missing end
/// is appended to the canonical part that exists
fn canonicalize_missing(path: &Path) -> std::io::Result<PathBuf> {
    let path = std::path::absolute(path)?;

    if let Ok(canonical) = path.canonicalize() {
        return Ok(canonical);
    }

    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => Ok(canonicalize_missing(parent)?.join(name)),
        _ => path.canonicalize(),
    }
}

/// Copies everything in `from` into `to`, creating `to` and the subdirectories as needed
fn copy_contents(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(Error::io("create the directory", to))?;

    for entry in std::fs::read_dir(from).map_err(Error::io("read", from))? {
        let entry = entry.map_err(Error::io("read", from))?;
        copy_entry(&entry.path(), &to.join(entry.file_name()), &entry)?;
    }

    Ok(())
}

/// Copies one directory entry: subdirectories recursively, symlinks as symlinks on unix
fn copy_entry(from: &Path, to: &Path, entry: &std::fs::DirEntry) -> Result<()> {
    #[cfg(unix)]
    if entry
        .file_type()
        .map_err(Error::io("read", from))?
        .is_symlink()
    {
        return copy_symlink(from, to);
    }
    #[cfg(not(unix))]
    let _ = entry;

    if from.is_dir() {
        copy_contents(from, to)
    } else {
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(Error::io("copy onto", to))
    }
}

/// Recreates the symlink `from` at `to`, replacing whatever symlink or file is there
#[cfg(unix)]
fn copy_symlink(from: &Path, to: &Path) -> Result<()> {
    let target = std::fs::read_link(from).map_err(Error::io("read", from))?;

    if to.symlink_metadata().is_ok_and(|meta| !meta.is_dir()) {
        std::fs::remove_file(to).map_err(Error::io("replace", to))?;
    }

    std::os::unix::fs::symlink(target, to).map_err(Error::io("create the link", to))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;
    use tempfile::TempDir;

    /// A source with `a.txt`, `sub/b.txt` and `sub/deeper/c.txt`, and an empty `to` path
    fn source() -> (TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from");
        fs::create_dir_all(from.join("sub").join("deeper")).unwrap();
        fs::write(from.join("a.txt"), "a").unwrap();
        fs::write(from.join("sub").join("b.txt"), "b").unwrap();
        fs::write(from.join("sub").join("deeper").join("c.txt"), "c").unwrap();
        let to = dir.path().join("to");
        (dir, from, to)
    }

    fn read(path: PathBuf) -> String {
        fs::read_to_string(path).unwrap()
    }

    #[test]
    fn it_always_runs() {
        let (_dir, from, to) = source();
        assert_eq!(Check::Drift, Copy { from, to }.check().unwrap());
    }

    #[test]
    fn it_copies_nested_files_as_contents_into_to() {
        let (_dir, from, to) = source();

        Copy {
            from,
            to: to.clone(),
        }
        .execute()
        .unwrap();

        assert_eq!("a", read(to.join("a.txt")));
        assert_eq!("b", read(to.join("sub").join("b.txt")));
        assert_eq!("c", read(to.join("sub").join("deeper").join("c.txt")));
        assert_eq!(false, to.join("from").exists());
    }

    #[test]
    fn it_treats_a_trailing_slash_on_to_the_same() {
        let (_dir, from, to) = source();
        let with_slash = PathBuf::from(format!("{}/", to.display()));

        Copy {
            from,
            to: with_slash,
        }
        .execute()
        .unwrap();

        assert_eq!("b", read(to.join("sub").join("b.txt")));
        assert_eq!(false, to.join("from").exists());
    }

    #[test]
    fn it_overwrites_files_and_keeps_unrelated_ones() {
        let (_dir, from, to) = source();
        fs::create_dir_all(to.join("sub")).unwrap();
        fs::write(to.join("a.txt"), "old").unwrap();
        fs::write(to.join("sub").join("mine.txt"), "mine").unwrap();

        Copy {
            from,
            to: to.clone(),
        }
        .execute()
        .unwrap();

        assert_eq!("a", read(to.join("a.txt")));
        assert_eq!("mine", read(to.join("sub").join("mine.txt")));
    }

    #[test]
    fn it_refuses_to_copy_into_a_directory_inside_from() {
        let (_dir, from, _to) = source();
        let inside = from.join("sub").join("backup");

        let error = Copy {
            from: from.clone(),
            to: inside.clone(),
        }
        .execute()
        .unwrap_err();

        assert!(error.to_string().contains("into itself"), "{error}");
        assert_eq!(false, inside.exists());
    }

    #[test]
    fn it_refuses_to_copy_a_directory_onto_itself() {
        let (_dir, from, _to) = source();
        let same = from.join("sub").join("..");

        let error = Copy {
            from: from.clone(),
            to: same,
        }
        .execute()
        .unwrap_err();

        assert!(error.to_string().contains("into itself"), "{error}");
        assert_eq!("a", read(from.join("a.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn it_refuses_a_to_that_reaches_into_from_through_a_symlink() {
        let (dir, from, _to) = source();
        let alias = dir.path().join("alias");
        std::os::unix::fs::symlink(&from, &alias).unwrap();

        let error = Copy {
            from,
            to: alias.join("sub").join("backup"),
        }
        .execute()
        .unwrap_err();

        assert!(error.to_string().contains("into itself"), "{error}");
    }

    #[test]
    fn it_copies_into_a_sibling_whose_name_starts_like_from() {
        let (_dir, from, _to) = source();
        let sibling = from.with_file_name("from-backup");

        Copy {
            from,
            to: sibling.clone(),
        }
        .execute()
        .unwrap();

        assert_eq!("a", read(sibling.join("a.txt")));
    }

    #[cfg(unix)]
    #[test]
    fn it_copies_symlinks_as_symlinks() {
        let (_dir, from, to) = source();
        std::os::unix::fs::symlink("a.txt", from.join("link")).unwrap();

        // twice, the second run replaces the existing link
        for _ in 0..2 {
            Copy {
                from: from.clone(),
                to: to.clone(),
            }
            .execute()
            .unwrap();
        }

        let link = to.join("link");
        assert_eq!(PathBuf::from("a.txt"), fs::read_link(&link).unwrap());
        assert_eq!("a", read(link));
    }
}
