//! The library behind comtrya: it loads manifests, gathers the facts about the system, and plans
//! the steps that bring the system to the state the manifests describe, and runs them. The
//! `comtrya` binary is the command line around it

#![warn(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    // A failure is an error, and `expect` is for what can't fail, saying why it holds
    clippy::unwrap_used
)]

pub mod actions;
pub mod atoms;
pub mod config;
pub mod contexts;
pub mod executor;
pub mod helper;
pub mod manifests;
pub mod runner;
mod runtime;
pub mod steps;
pub mod tera_functions;
mod utilities;
pub mod values;

pub use utilities::error_chain;
