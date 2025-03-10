use std::{fs::create_dir_all, io, path::PathBuf};

use super::ComtryaCommand;
use crate::Runtime;

use anyhow::Result;
use clap::{Command, CommandFactory, Parser, Subcommand};
use gix::prelude::*;

use crate::GlobalArgs;

#[derive(Subcommand, Debug)]
pub enum PluginSubCommands {
    /// Install a plugin
    Add { name: String },
    /// List installed plugins
    List,
    /// Remove a plugin
    Remove { name: String },
    /// Update plugins
    Update,
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
pub struct PluginCommands {
    #[command(subcommand)]
    pub command: PluginSubCommands,
}

impl ComtryaCommand for PluginCommands {
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        match &self.command {
            PluginSubCommands::Add { name } => {
                // Implement install logic
                println!("Installing plugin: {}", name);
                Ok(())
            }
            PluginSubCommands::List => {
                // Implement list logic
                println!("Listing installed plugins");
                Ok(())
            }
            PluginSubCommands::Remove { name } => {
                println!("Removing plugin: {}", name);
                Ok(())
            }
            PluginSubCommands::Update => {
                println!("Updating plugins");
                Ok(())
            }
        }
    }
}
