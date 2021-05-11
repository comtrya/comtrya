use super::super::Atom;
use super::finalizers;
use super::initializers;
use super::CommandAtom;

pub struct CommandRun {
    command: String,
    arguments: Vec<String>,
    privileged: bool,
    initializers: Vec<initializers::FlowControl>,
    finalizers: Vec<finalizers::FlowControl>,
}

impl CommandAtom for CommandRun {}

impl std::fmt::Display for CommandRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RunCommand with privileged {}: {} {}",
            self.privileged,
            self.command,
            self.arguments.join(" ")
        )
    }
}

impl Atom for CommandRun {
    fn plan(&self) -> bool {
        let mut initializers = self.initializers.iter();

        while let Some(initializer) = initializers.next() {
            match initializer {
                initializers::FlowControl::SkipIf(skip) => {
                    if skip.run() {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn execute(&self) -> anyhow::Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializers() {
        use super::initializers::command_found::CommandFound;
        use super::initializers::FlowControl::SkipIf;

        // Ensure that no initializers always returns true
        let command_run = CommandRun {
            command: String::from("echo"),
            arguments: vec![],
            privileged: false,
            initializers: vec![],
            finalizers: vec![],
        };

        assert_eq!(true, command_run.plan());

        // Ensure that SkipIf returns false when satisfied
        let command_run = CommandRun {
            command: String::from("echo"),
            arguments: vec![],
            privileged: false,
            initializers: vec![
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
                SkipIf(Box::new(CommandFound("ls"))),
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
            ],
            finalizers: vec![],
        };

        assert_eq!(false, command_run.plan());

        // Ensure that SkipIf returns true when not satisfied
        let command_run = CommandRun {
            command: String::from("echo"),
            arguments: vec![],
            privileged: false,
            initializers: vec![
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
            ],
            finalizers: vec![],
        };

        assert_eq!(true, command_run.plan())
    }
}
