use crate::atoms::Atom;

mod env_vars_remove;
mod output_contains;
pub use env_vars_remove::RemoveEnvVars;

pub use output_contains::OutputContains;

#[allow(dead_code)]
pub enum FlowControl {
    Ensure(Box<dyn Finalizer>),
    StopIf(Box<dyn Finalizer>),
}

/// Finalizers allow us to store data within the manifests KV store,
/// or to end the execution of atoms for the action
pub trait Finalizer: Send + Sync {
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
