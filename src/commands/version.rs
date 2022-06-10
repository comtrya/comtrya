use crate::config::Config;
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub(crate) struct Version {}

pub(crate) fn execute(args: Version, config: Config) -> anyhow::Result<()> {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!("{}", VERSION.unwrap_or("unknown"));

    Ok(())
}
