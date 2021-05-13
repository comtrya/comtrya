use super::Atom;

mod exec;
pub mod finalizers;
pub mod initializers;

pub use exec::Exec;

pub trait CommandAtom: Atom {}
