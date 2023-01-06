use super::super::Atom;
use super::FileAtom;
use file_diff::diff;
use std::path::PathBuf;
use tracing::error;

pub struct Copy {
    pub from: PathBuf,
    pub to: PathBuf,
}

impl FileAtom for Copy {
    fn get_path(&self) -> &PathBuf {
        &self.from
    }
}

impl std::fmt::Display for Copy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be copied from {}",
            self.to.display(),
            self.from.display(),
        )
    }
}

impl Atom for Copy {
    fn plan(&self) -> bool {
        if !self.to.is_file() {
            error!("Cannot plan: target isn't a file: {}", self.to.display());

            return false;
        }

        !diff(
            &self.from.display().to_string(),
            &self.to.display().to_string(),
        )
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::copy(&self.from, &self.to)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;

    #[test]
    fn it_can_plan() {
        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(true, file_copy.plan());

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: from_file.path().to_path_buf(),
        };

        assert_eq!(false, file_copy.plan());
    }

    #[test]
    fn it_can_execute() {
        use std::io::Write;

        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let mut from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        assert_eq!(
            true,
            writeln!(
                from_file.as_file_mut(),
                "This is test content for a test file"
            )
            .is_ok()
        );

        let mut file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to_file.path().to_path_buf(),
        };

        assert_eq!(true, file_copy.plan());
        assert_eq!(true, file_copy.execute().is_ok());
        assert_eq!(false, file_copy.plan());
    }

    #[test]
    fn it_wont_destroy_directories() {
        let to = match tempdir::TempDir::new("file-copy") {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let from_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_copy = Copy {
            from: from_file.path().to_path_buf(),
            to: to.path().to_path_buf(),
        };

        assert_eq!(false, file_copy.plan());
    }
}
