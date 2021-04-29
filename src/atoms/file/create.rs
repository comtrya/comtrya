use super::super::Atom;
use super::FileAtom;
use std::path::PathBuf;

pub struct FileCreate {
    path: PathBuf,
}

impl FileAtom for FileCreate {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
}

impl Atom for FileCreate {
    fn plan(&self) -> bool {
        !self.path.exists()
    }

    fn execute(&self) -> anyhow::Result<()> {
        std::fs::File::create(&self.path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_plan() {
        let file_create = FileCreate {
            path: std::path::PathBuf::from("some-random-path"),
        };

        assert_eq!(true, file_create.plan());

        let temp_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_create = FileCreate {
            path: temp_file.path().to_path_buf(),
        };

        assert_eq!(false, file_create.plan());
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

        let file_create = FileCreate {
            path: temp_dir.path().join("create-me"),
        };

        assert_eq!(true, file_create.execute().is_ok());
        assert_eq!(false, file_create.plan());
    }

    #[test]
    fn it_can_revert() {
        let temp_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let file_create = FileCreate {
            path: temp_file.path().to_path_buf(),
        };

        assert_eq!(true, file_create.revert().is_ok());
        assert_eq!(true, file_create.plan());
    }
}
