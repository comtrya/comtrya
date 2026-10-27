//! Downloading files

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use super::Atom;

mod download;
pub use download::Download;

/// Atoms that use HTTP. Nothing implements it yet
pub trait HttpAtom: Atom {}
