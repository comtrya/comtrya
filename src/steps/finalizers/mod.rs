use crate::atoms::Atom;

mod output_contains;

#[allow(dead_code)]
pub enum FlowControl {
    StopIf(Box<dyn Finalizer>),
}

/// Finalizers allow us to store data within the manifests KV store,
/// or to end the execution of atoms for the action
pub trait Finalizer {
    fn finalize(&self, atom: Box<dyn Atom>) -> anyhow::Result<bool>;
}
