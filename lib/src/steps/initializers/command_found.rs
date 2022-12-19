use super::Initializer;

#[derive(Clone, Debug)]
pub struct CommandFound(pub &'static str);

impl Initializer for CommandFound {
    fn initialize(&self) -> anyhow::Result<bool> {
        Ok(which::which(self.0).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_returns_false_when_not_found() {
        let initializer = CommandFound("not-a-real-command");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(false, result.unwrap());
    }

    #[cfg(target_family = "windows")]
    #[test]
    fn it_returns_true_when_found() {
        let initializer = CommandFound("cmd.exe");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
    
    #[cfg(target_family = "windows")]
    #[test]
    fn return_true_windows_xcopy() {
        let initializer = CommandFound("Xcopy");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }

    #[cfg(target_family = "unix")]
    #[test]
    fn it_returns_true_when_found() {
        let initializer = CommandFound("ls");
        let result = initializer.initialize();

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
}
