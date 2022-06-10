use std::path::PathBuf;

use super::Initializer;

#[derive(Clone, Debug)]
pub struct FileExists(pub PathBuf);

impl Initializer for FileExists {
    fn initialize(&self) -> anyhow::Result<bool> {
        Ok(self.0.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_false_when_not_found() {
        let initializer = FileExists(PathBuf::from("not-a-existing-file"));
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(false, result.unwrap());
    }

    #[test]
    fn it_returns_true_when_found() {
        let to_file = match tempfile::NamedTempFile::new() {
            std::result::Result::Ok(file) => file,
            std::result::Result::Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let path_buf = to_file.path().to_path_buf();

        let initializer = FileExists(path_buf);
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
}
