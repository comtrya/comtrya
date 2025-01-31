use super::ComtryaCommand;
use crate::Runtime;
use colored::Colorize;
use comfy_table::{presets::NOTHING, Attribute, Cell, ContentArrangement, Table};

use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command()]
pub(crate) struct Contexts {
    /// Show the values of the contexts
    #[arg(long)]
    show_values: bool,
}

impl ComtryaCommand for Contexts {
    async fn execute(&self, runtime: &mut Runtime) -> anyhow::Result<()> {
        for (name, context) in runtime.contexts.iter() {
            println!("{}", name.to_string().underline().bold());

            let mut table = Table::new();
            table
                .load_preset(NOTHING)
                .set_content_arrangement(ContentArrangement::Dynamic);

            if context.is_empty() {
                table.add_row(vec![Cell::new("<empty>")]);
                println!("{table}");
                println!();

                continue;
            }

            // Only show keys, when flag is not set
            if !self.show_values {
                let chunk_size = if context.len() > 10 { 6 } else { 1 };

                context
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                    .chunks(chunk_size)
                    .for_each(|chunk| {
                        let mut row = vec![];

                        for key in chunk {
                            row.push(key);
                        }

                        table.add_row(row);
                    });
            } else {
                for (key, value) in context.iter() {
                    let value = strip_ansi_escapes::strip(value.to_string());
                    let value = String::from_utf8(value).unwrap_or_default();

                    table.add_row(vec![
                        Cell::new(key).add_attribute(Attribute::Bold),
                        Cell::new(value),
                    ]);
                }
            }

            println!("{table}");

            println!();
        }

        Ok(())
    }
}
