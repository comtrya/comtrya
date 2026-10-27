//! The subcommands of `comtrya`

mod apply;
pub(crate) use apply::Apply;

/// `comtrya version`
mod version;
pub(crate) use version::Version;

/// `comtrya contexts`
mod contexts;
pub(crate) use contexts::Contexts;

/// `comtrya gen-completions`
mod gen_completions;
pub(crate) use gen_completions::GenCompletions;

use crate::Runtime;

/// A subcommand
pub trait ComtryaCommand {
    /// Runs it
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}
