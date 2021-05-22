use super::Finalizer;
use crate::atoms::Atom;

#[derive(Clone, Debug)]
pub struct OutputContains(pub &'static str);

impl Finalizer for OutputContains {
    fn finalize(&self, atom: &dyn Atom) -> anyhow::Result<bool> {
        Ok(atom.output_string().contains(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Echo;

    #[test]
    fn it_returns_false_when_not_found() {
        let atom = Echo("goodbye-world");
        let finalizer = OutputContains("hello-world");
        let result = finalizer.finalize(&atom);

        assert_eq!(true, result.is_ok());
        assert_eq!(false, result.unwrap());
    }

    #[test]
    fn it_returns_true_when_found() {
        let step = Echo("hello-world");
        let finalizer = OutputContains("hello-world");
        let result = finalizer.finalize(&step);

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
}
