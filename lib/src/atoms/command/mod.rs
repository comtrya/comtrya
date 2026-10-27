//! Running programs

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use super::Atom;

mod exec;
pub use exec::Exec;

/// Atoms that run programs. Nothing implements it yet
pub trait CommandAtom: Atom {}
