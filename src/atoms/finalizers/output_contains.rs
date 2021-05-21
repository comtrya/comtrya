use super::Finalizer;
use crate::atoms::Atom;

#[derive(Clone, Debug)]
pub struct OutputContains(pub &'static str);

impl Finalizer for OutputContains {
    fn finalize(&self, atom: Box<dyn Atom>) -> anyhow::Result<bool> {
        Ok(atom.output_string().contains(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Echo;

    #[test]
    fn it_returns_false_when_not_found() {
        let finalizer = OutputContains("hello-world");
        let result = finalizer.finalize(Echo::boxed("goodbye-world"));

        assert_eq!(true, result.is_ok());
        assert_eq!(false, result.unwrap());
    }

    #[test]
    fn it_returns_true_when_found() {
        let finalizer = OutputContains("hello-world");
        let result = finalizer.finalize(Echo::boxed("hello-world"));

        assert_eq!(true, result.is_ok());
        assert_eq!(true, result.unwrap());
    }
}
