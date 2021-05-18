use super::Finalizer;

#[derive(Clone, Debug)]
pub struct OutputContains(pub &'static str);

impl Finalizer for OutputContains {
    fn run(&self, result: &Result<std::process::Output, std::io::Error>) -> bool {
        match result {
            Ok(std::process::Output { stdout, .. }) => {
                // Command run OK, check for removed
                let out_string = String::from_utf8(stdout.clone()).unwrap();
                out_string.to_lowercase().contains(self.0)
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::atoms::{
        command::{finalizers::FlowControl, Exec},
        Atom,
    };

    use super::*;

    #[test]
    fn it_returns_true() {
        assert_eq!(
            true,
            Exec {
                command: String::from("echo"),
                arguments: vec![String::from("random-string")],
                finalizers: vec![FlowControl::ErrorIf(Box::new(OutputContains(
                    "random-string"
                )))],
                ..Default::default()
            }
            .execute()
            .is_err()
        );

        assert_eq!(
            true,
            Exec {
                command: String::from("echo"),
                arguments: vec![String::from("random-string")],
                // Should finish before ErrorIf
                finalizers: vec![
                    FlowControl::FinishIf(Box::new(OutputContains("random-string"))),
                    FlowControl::ErrorIf(Box::new(OutputContains("random-string")))
                ],
                ..Default::default()
            }
            .execute()
            .is_ok()
        );
    }
}
