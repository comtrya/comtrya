use super::Initializer;

#[derive(Clone, Debug)]
pub struct CommandFound(pub &'static str);

impl Initializer for CommandFound {
    fn run(&self) -> bool {
        use which::which;
        which(self.0).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_true_when_not_found() {
        let command_found = CommandFound("not-a-real-command");

        assert_eq!(false, command_found.run())
    }

    #[test]
    fn it_returns_false_when_found() {
        let command_found = CommandFound("ls");

        assert_eq!(true, command_found.run())
    }
}
