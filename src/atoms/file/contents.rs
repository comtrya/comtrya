use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::error;

pub struct FileContents {
    path: PathBuf,
    contents: String,
}

impl FileAtom for FileContents {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for FileContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents needs to be set to {}",
            self.path.to_str().unwrap(),
            self.contents,
        )
    }
}

impl Atom for FileContents {
    fn plan(&self) -> bool {
        let contents = match std::fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) => {
                error!(
                    "Failed to read contents of {} for diff because {:?}. Skipping",
                    error, self.path
                );

                return false;
            }
        };

        !contents.eq(&self.contents)
    }

    fn execute(&self) -> anyhow::Result<()> {
        std::fs::write(&self.path, &self.contents)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can() {
        let file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_contents = FileContents {
            path: file.path().to_path_buf(),
            contents: String::from(""),
        };

        assert_eq!(false, file_contents.plan());

        let file_contents = FileContents {
            path: file.path().to_path_buf(),
            contents: String::from("Hello, world!"),
        };

        assert_eq!(true, file_contents.plan());
        assert_eq!(true, file_contents.execute().is_ok());
        assert_eq!(false, file_contents.plan());
    }
}
