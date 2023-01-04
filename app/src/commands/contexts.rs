use super::ComtryaCommand;
use crate::Runtime;
use colored::Colorize;
use ptree::{print_tree, TreeBuilder};
use structopt::StructOpt;

#[derive(Clone, Debug, StructOpt)]
pub(crate) struct Contexts {}

impl ComtryaCommand for Contexts {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        for (name, context) in runtime.contexts.iter() {
            let mut tree_builder =
                TreeBuilder::new(format!("{}", name.to_string().underline().bold()));

            if context.is_empty() {
                tree_builder.begin_child("<empty>".to_string());
                tree_builder.end_child();
            } else {
                for (name, value) in context.iter() {
                    let value = strip_ansi_escapes::strip(value.to_string())?;
                    let value = std::str::from_utf8(&value)?;

                    tree_builder.begin_child(format!("{}: {}", name, value));
                    tree_builder.end_child();
                }
            }

            print_tree(&tree_builder.build())?;

            println!();
        }

        Ok(())
    }
}
