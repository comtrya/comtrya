use super::ComtryaCommand;
use crate::Runtime;

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command()]
pub(crate) struct Version {}

impl ComtryaCommand for Version {
    async fn execute(&self, _: &mut Runtime) -> anyhow::Result<()> {
        const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
        println!("{}", VERSION.unwrap_or("unknown"));

        Ok(())
    }
}
