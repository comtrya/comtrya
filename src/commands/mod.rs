use structopt::StructOpt;

mod apply;
pub(crate) use apply::*;
mod version;
pub(crate) use version::*;

use crate::config::Config;

#[derive(Clone, Debug, StructOpt)]
pub(crate) enum Commands {
    #[structopt(aliases = &["do", "run"])]
    Apply(apply::Apply),
    Version(version::Version),
}

#[derive(Clone, structopt::StructOpt)]
#[structopt(name = "comtrya")]
pub(crate) struct Args {
    /// Directory
    #[structopt(short = "d", long)]
    pub manifest_directory: Option<String>,

    /// Disable color printing
    #[structopt(long = "no-color")]
    pub no_color: bool,

    /// Debug & tracing mode (-v, -vv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(subcommand)]
    pub command: Commands,
}

pub(crate) fn execute(args: Args, config: Config) -> anyhow::Result<()> {
    match args.command {
        Commands::Apply(a) => apply::execute(a, config),
        Commands::Version(a) => version::execute(a, config),
    }
}
