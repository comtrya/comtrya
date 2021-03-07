use crate::manifests::Manifest;

mod command;

pub enum Actions {}

pub struct ActionResult {
    /// Output / response
    message: String,
}
pub struct ActionError {
    /// Error message
    message: String,
}

pub trait Action {
    fn action(self: Self, manifest: &Manifest) -> Result<ActionResult, ActionError>;
}
