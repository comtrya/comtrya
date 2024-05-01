use clap::Subcommand;

mod apply;
pub(crate) use apply::Apply;

mod version;
pub(crate) use version::Version;

mod contexts;
pub(crate) use contexts::Contexts;

mod gen_completions;
pub(crate) use gen_completions::GenCompletions;

use crate::Runtime;

pub trait ComtryaCommand {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()>;
}

#[derive(Debug, Subcommand, PartialEq)]
pub enum Commands {
    /// Apply manifests
    #[clap(aliases = &["do", "run"])]
    Apply(Apply),

    /// Print version information
    Version(Version),

    /// List available contexts (BETA)
    Contexts(Contexts),

    /// Auto generate completions
    ///
    /// for examples:
    ///  - bash: ```source <(comtrya gen-completions bash)```
    ///  - fish: ```comtrya gen-completions fish | source```
    #[command(long_about, verbatim_doc_comment)]
    GenCompletions(GenCompletions),
}

impl Commands {
    pub fn execute(self, runtime: &Runtime) -> anyhow::Result<()> {
        match self {
            Self::Apply(apply) => apply.execute(&runtime),
            Self::Version(version) => version.execute(&runtime),
            Self::Contexts(contexts) => contexts.execute(&runtime),
            Self::GenCompletions(gen_completions) => gen_completions.execute(&runtime),
        }
    }
}
