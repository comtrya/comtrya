//! Finalizers: checks after an atom ran, which can stop the rest of the action

use crate::atoms::Atom;

mod output_contains;

pub use output_contains::OutputContains;

/// How a finalizer's answer is used
#[allow(dead_code)]
pub enum FlowControl {
    /// The action goes on only if the finalizer says yes
    Ensure(Box<dyn Finalizer>),
    /// The action stops if the finalizer says yes
    StopIf(Box<dyn Finalizer>),
}

/// A check after an atom ran, which can end the action
pub trait Finalizer {
    /// The answer, from what the atom returned
    fn finalize(&self, atom: &dyn Atom) -> anyhow::Result<bool>;
}

/// Finalizers for tests
#[cfg(test)]
pub mod test {
    use super::*;
    use anyhow::anyhow;

    /// Answers with its value
    pub struct EchoFinalizer(pub bool);

    impl Finalizer for EchoFinalizer {
        fn finalize(&self, _atom: &dyn Atom) -> anyhow::Result<bool> {
            Ok(self.0)
        }
    }

    /// Fails
    pub struct ErrorFinalizer();

    impl Finalizer for ErrorFinalizer {
        fn finalize(&self, _atom: &dyn Atom) -> anyhow::Result<bool> {
            Err(anyhow!("ErrorFinalizer"))
        }
    }
}
