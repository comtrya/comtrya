//! Copies the contents of a directory into another one

use crate::atoms::Outcome;

use super::super::Atom;
use std::path::{Path, PathBuf};

/// Copies the contents of a directory into another one, recursively.
///
/// `to` is created if missing. Files that exist in both are overwritten, files only in `to` are
/// kept. Symlinks are recreated as symlinks on unix; elsewhere the file they point to is copied.
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

impl Atom for Copy {
    /// Always runs: comparing every file would cost about as much as copying them
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        copy_contents(&self.from, &self.to)?;

        Ok(())
    }
}

/// Copies everything in `from` into `to`, creating `to` and the subdirectories as needed
fn copy_contents(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;

    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        copy_entry(&entry.path(), &to.join(entry.file_name()), &entry)?;
    }

    Ok(())
}

/// Copies one directory entry: subdirectories recursively, symlinks as symlinks on unix
fn copy_entry(from: &Path, to: &Path, entry: &std::fs::DirEntry) -> std::io::Result<()> {
    #[cfg(unix)]
    if entry.file_type()?.is_symlink() {
        return copy_symlink(from, to);
    }
    #[cfg(not(unix))]
    let _ = entry;

    if from.is_dir() {
        copy_contents(from, to)
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

/// Recreates the symlink `from` at `to`, replacing whatever symlink or file is there
#[cfg(unix)]
fn copy_symlink(from: &Path, to: &Path) -> std::io::Result<()> {
    let target = std::fs::read_link(from)?;

    if to.symlink_metadata().is_ok_and(|meta| !meta.is_dir()) {
        std::fs::remove_file(to)?;
    }

    std::os::unix::fs::symlink(target, to)
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
        assert_eq!(true, Copy { from, to }.plan().unwrap().should_run);
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
