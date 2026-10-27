//! Creating and removing directories

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

mod create;
mod remove;
pub use create::Create;
pub use remove::Remove;
