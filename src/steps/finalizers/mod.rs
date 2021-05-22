use crate::atoms::Atom;

mod output_contains;
pub use output_contains::OutputContains;

#[allow(dead_code)]
pub enum FlowControl {
    StopIf(Box<dyn Finalizer>),
}

/// Finalizers allow us to store data within the manifests KV store,
/// or to end the execution of atoms for the action
pub trait Finalizer {
    fn finalize(&self, atom: &dyn Atom) -> anyhow::Result<bool>;
}

#[cfg(test)]
pub mod test {
    use super::*;
    use anyhow::anyhow;

    pub struct EchoFinalizer(pub bool);

    impl Finalizer for EchoFinalizer {
        fn finalize(&self, _atom: &dyn Atom) -> anyhow::Result<bool> {
            Ok(self.0)
        }
    }

    pub struct ErrorFinalizer();

    impl Finalizer for ErrorFinalizer {
        fn finalize(&self, _atom: &dyn Atom) -> anyhow::Result<bool> {
            Err(anyhow!("ErrorFinalizer"))
        }
    }
}
