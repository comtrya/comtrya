//! Downloading files

use super::Atom;

mod download;
pub use download::Download;

/// Atoms that use HTTP. Nothing implements it yet
pub trait HttpAtom: Atom {}
