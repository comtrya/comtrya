use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;
use tracing::error;

pub struct SetContents {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

impl FileAtom for SetContents {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl std::fmt::Display for SetContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The file {} contents need to be set",
            self.path.to_str().unwrap(),
        )
    }
}

impl Atom for SetContents {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                should_run: true,
                side_effects: Vec::new(),
            });
        }

        let contents = match std::fs::read(&self.path) {
            Ok(contents) => contents,
            Err(error) => {
                error!(
                    "Failed to read contents of {} for diff because {:?}. Skipping",
                    error, self.path
                );

                return Ok(Outcome {
                    should_run: false,
                    side_effects: Vec::new(),
                });
            }
        };

        Ok(Outcome {
            should_run: !contents.eq(&self.contents),
            side_effects: Vec::new(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::write(&self.path, &self.contents)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can() {
        let file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("").into_bytes(),
        };

        assert_eq!(false, file_contents.plan().unwrap().should_run);

        let mut file_contents = SetContents {
            path: file.path().to_path_buf(),
            contents: String::from("Hello, world!").into_bytes(),
        };

        assert_eq!(true, file_contents.plan().unwrap().should_run);
        assert_eq!(true, file_contents.execute().is_ok());
        assert_eq!(false, file_contents.plan().unwrap().should_run);
    }
}
