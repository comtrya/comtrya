use std::path::PathBuf;

use tracing::error;

use crate::atoms::{Atom, Outcome};

pub struct Remove {
    pub target: PathBuf,
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The directory {} needs to be removed",
            self.target.display(),
        )
    }
}

impl Atom for Remove {
    fn plan(&self) -> anyhow::Result<Outcome> {
        let path_to_dir = PathBuf::from(&self.target);

        if !path_to_dir.is_dir() {
            error!(
                "Cannot plan: target isn`t a directory: {}",
                self.target.display()
            );
            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        let is_empty = path_to_dir.read_dir()?.next().is_none();

        if !is_empty {
            error!(
                "Cannot plan: directory {} is not empty",
                self.target.display()
            );

            return Ok(Outcome {
                side_effects: vec![],
                should_run: false,
            });
        }

        Ok(Outcome {
            side_effects: vec![],
            should_run: self.target.exists(),
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::remove_dir(&self.target)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::{fs::File, io::Write};

    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn it_can_plan() {
        let temp = tempdir().unwrap().keep();
        let atom = Remove {
            target: temp.clone(),
        };
        assert_eq!(true, atom.plan().unwrap().should_run);

        let file_path = temp.join("my-temporary-note.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Brian was here. Briefly.").unwrap();

        // Should not run when dir is not empty
        assert_eq!(false, atom.plan().unwrap().should_run);

        // Should not run if path is not a dir
        let atom = Remove {
            target: file_path.clone(),
        };

        assert_eq!(false, atom.plan().unwrap().should_run)
    }

    #[test]
    fn it_can_execute() {
        let temp_dir = match tempfile::tempdir() {
            std::result::Result::Ok(dir) => dir,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let path = temp_dir.path();

        assert_eq!(true, path.exists());

        let mut atom = Remove {
            target: temp_dir.path().to_path_buf(),
        };

        // Deletes dir
        assert_eq!(true, atom.execute().is_ok());
        // Dir is deleted and dont exists
        assert_eq!(false, temp_dir.path().exists())
    }
}
