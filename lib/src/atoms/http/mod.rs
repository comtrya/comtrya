use super::Atom;

mod download;
pub use download::Download;

pub trait HttpAtom: Atom {}
