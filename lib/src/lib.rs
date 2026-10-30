//! The library behind comtrya: it loads manifests, gathers the facts about the system, and plans
//! the steps that bring the system to the state the manifests describe, and runs them. The
//! `comtrya` binary is the command line around it

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

pub mod actions;
pub mod atoms;
pub mod config;
pub mod contexts;
pub mod executor;
pub mod manifests;
pub mod runner;
pub mod steps;
pub mod tera_functions;
mod utilities;
pub mod values;
