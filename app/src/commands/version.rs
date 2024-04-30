use super::ComtryaCommand;
use crate::Runtime;

use clap::Parser;

#[derive(Parser, Debug)]
#[command()]
pub(crate) struct Version {}

impl ComtryaCommand for Version {
    fn execute(&self, _: &Runtime) -> anyhow::Result<()> {
        const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
        println!("{}", VERSION.unwrap_or("unknown"));

        Ok(())
    }
}
