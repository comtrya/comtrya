//! The library behind comtrya: it loads manifests, gathers the facts about the system, and plans
//! the steps that bring the system to the state the manifests describe, and runs them. The
//! `comtrya` binary is the command line around it.
//!
//! # The path of `comtrya apply`
//!
//! 1. [`config`] reads `Comtrya.yaml`, and [`contexts`] gathers the [`Facts`](contexts::Facts):
//!    the user, the OS, the environment, variables. As [`Contexts`](contexts::Contexts) they are
//!    what templates and `where` conditions see.
//! 2. [`manifests`] finds the manifest directory, renders each file's templates with the
//!    contexts, and parses it into a [`Manifest`](manifests::Manifest): a list of actions.
//! 3. [`actions`]: each action, like `file.link`, picks the variant that applies and plans
//!    [`Step`](steps::Step)s, without changing anything.
//! 4. [`steps`] and [`atoms`]: a step wraps an [`Atom`](atoms::Atom), the smallest change, like
//!    creating a symlink, and says who makes it, the user or root.
//! 5. [`executor`] runs the manifests, dependencies first. It runs each step whose atom's check
//!    says the system differs, and tells an [`Observer`](executor::Observer) what happens as
//!    [`Event`](executor::Event)s. The command line turns them into its output.
//!
//! The same path in code, for a manifest directory with one manifest:
//!
//! ```
//! use comtrya_lib::actions::PlanCx;
//! use comtrya_lib::contexts::Facts;
//! use comtrya_lib::executor::{self, Options};
//! use comtrya_lib::manifests::load_all;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let manifests = tempfile::tempdir()?;
//! let made = manifests.path().join("made");
//! let manifest = format!(
//!     "actions:\n  - action: directory.create\n    path: '{}'\n",
//!     made.display()
//! );
//! std::fs::write(manifests.path().join("demo.yaml"), manifest)?;
//!
//! let facts = Facts::default(); // `Facts::gather` looks at the real system
//! let contexts = facts.to_contexts();
//! let loaded = load_all(manifests.path().to_path_buf(), &contexts);
//! let cx = PlanCx::new(&facts, &contexts);
//! let report = executor::run(loaded, manifests.path(), cx, &Options::default());
//!
//! report.into_result()?;
//! assert!(made.is_dir());
//! # Ok(())
//! # }
//! ```
//!
//! [`helper`] is the other half of root steps: the privileged process that runs their atoms.

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
pub mod registered;
pub mod runner;
pub mod runtime;
pub mod steps;
pub mod tera_functions;
pub mod tls;
mod utilities;
pub mod values;

pub use utilities::error_chain;
