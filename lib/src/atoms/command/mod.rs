//! Running programs

use super::Atom;

mod exec;
pub use exec::Exec;

/// Atoms that run programs. Nothing implements it yet
pub trait CommandAtom: Atom {}
