use super::Atom;

mod finalizers;
mod initializers;
mod run;

pub trait CommandAtom: Atom {}
