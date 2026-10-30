//! Checks the output of an atom

use super::Finalizer;
use crate::atoms::Output;

/// Says yes if the atom's stdout contains the text
#[derive(Clone, Debug)]
pub struct OutputContains(pub &'static str);

impl Finalizer for OutputContains {
    fn finalize(&self, output: &Output) -> anyhow::Result<bool> {
        Ok(output.stdout.contains(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn stdout(text: &str) -> Output {
        Output {
            stdout: text.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn it_returns_false_when_not_found() {
        let result = OutputContains("hello-world").finalize(&stdout("goodbye-world"));

        assert_eq!(false, result.unwrap());
    }

    #[test]
    fn it_returns_true_when_found() {
        let result = OutputContains("hello-world").finalize(&stdout("hello-world"));

        assert_eq!(true, result.unwrap());
    }
}
