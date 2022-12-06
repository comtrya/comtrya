use super::super::Atom;
use std::path::PathBuf;

pub struct Create {
    pub path: PathBuf,
}

impl std::fmt::Display for Create {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The directory {} needs to be created",
            self.path.display(),
        )
    }
}

impl Atom for Create {
    fn plan(&self) -> bool {
        !self.path.exists()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_can_plan() {
        let atom = Create {
            path: std::path::PathBuf::from("/some-random-path"),
        };
        assert_eq!(true, atom.plan());

        let temp = temp_dir();
        let atom = Create { path: temp };
        assert_eq!(false, atom.plan());
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

        let mut atom = Create {
            path: temp_dir.path().join("create-me"),
        };

        assert_eq!(false, temp_dir.path().join("create-me").exists());

        assert_eq!(true, atom.execute().is_ok());
        assert_eq!(false, atom.plan());

        assert_eq!(true, temp_dir.path().join("create-me").is_dir());
    }
}
