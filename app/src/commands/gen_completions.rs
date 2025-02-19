use std::io;

use super::ComtryaCommand;
use crate::Runtime;

use clap::{Command, CommandFactory, Parser};
use clap_complete::{generate, Generator, Shell};

use crate::GlobalArgs;

#[derive(Parser, Debug, Clone)]
#[command(arg_required_else_help = true)]
pub(crate) struct GenCompletions {
    /// If provided, outputs the completion file for given shell
    #[arg(value_enum)]
    shell: Shell,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

impl ComtryaCommand for GenCompletions {
    async fn execute(&self, _runtime: &mut Runtime) -> anyhow::Result<()> {
        print_completions(self.shell, &mut GlobalArgs::command());

        Ok(())
    }
}
