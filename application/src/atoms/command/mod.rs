use super::Atom;

mod exec;
pub use exec::Exec;

pub trait CommandAtom: Atom {}
