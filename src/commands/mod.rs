use structopt::StructOpt;

mod version;
pub(crate) use version::*;

#[derive(Clone, Debug, StructOpt)]
pub(crate) enum Commands {
    Version(version::Version),
}

#[derive(Clone, structopt::StructOpt)]
#[structopt(name = "comtrya")]
pub(crate) struct Args {
    /// Disable color printing
    #[structopt(long = "no-color")]
    pub no_color: bool,

    /// Debug & tracing mode (-v, -vv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    pub verbose: u8,

    #[structopt(subcommand)]
    pub command: Commands,
}

pub(crate) fn execute(args: Args) -> anyhow::Result<()> {
    match args.command {
        Commands::Version(a) => version::execute(a),
    }
}
