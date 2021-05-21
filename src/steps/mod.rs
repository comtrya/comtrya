use crate::atoms::Atom;
use std::fmt::Display;

pub mod finalizers;
pub mod initializers;

pub struct Step {
    pub atom: Box<dyn Atom>,
    pub initializers: Vec<initializers::FlowControl>,
    pub finalizers: Vec<finalizers::FlowControl>,
}

impl Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Step: {} (Not printing initializers and finalizers yet)",
            self.atom
        )
    }
}
