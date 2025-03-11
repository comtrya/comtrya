use std::{
    ffi::OsStr,
    fs::{self, create_dir_all},
    path::PathBuf,
};

use super::ComtryaCommand;
use crate::Runtime;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use dirs_next::data_local_dir;
use gix::{
    interrupt::IS_INTERRUPTED, open as open_repo, progress::Discard,
    remote::ref_map::Options as GixOptions, remote::Direction,
};

fn plugin_path() -> PathBuf {
    data_local_dir().unwrap().join("comtrya").join("plugins")
}

#[derive(Subcommand, Debug)]
pub enum PluginSubCommands {
    /// Install a plugin
    #[command(short_flag = 'a')]
    Add { name: String },
    /// List installed plugins
    #[command(short_flag = 'l')]
    List,
    /// Remove a plugin
    #[command(short_flag = 'r')]
    Remove { name: String },
    /// Update plugins
    #[command(short_flag = 'u')]
    Update { name: Option<String> },
}

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
pub struct PluginCommands {
    #[command(subcommand)]
    pub command: PluginSubCommands,
}

fn add_plugin(name: &str) -> Result<()> {
    let plugin_path = plugin_path().join(name);

    if plugin_path.exists() {
        return Err(anyhow!("Plugin {} already loaded", name));
    }

    let url = format!("https://github.com/{}", name);

    let _ = gix::prepare_clone(url.as_str(), &plugin_path)
        .context("Cannot find repository for plugin")?
        .fetch_then_checkout(gix::progress::Discard, &IS_INTERRUPTED)?;

    Ok(())
}

fn list_plugins() -> Result<()> {
    let plugin_dir = plugin_path();

    if fs::metadata(&plugin_dir).is_ok() || fs::read_dir(&plugin_dir)?.next().is_none() {
        println!("No plugins found");
        return Ok(());
    }

    for entry in fs::read_dir(plugin_dir)?.filter_map(Result::ok) {
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(OsStr::to_str)
                .context("Invalid path")?;
            if name != ".git" {
                println!("Plugin: {}", name);
            }
        }
    }
    Ok(())
}

fn remove_plugin(name: &str) -> Result<()> {
    let plugin_path = plugin_path().join(name);

    if plugin_path.exists() {
        fs::remove_dir_all(&plugin_path)?;
        println!("Plugin removed: {}", name);
    } else {
        println!("Plugin {} does not exist", name);
    }

    Ok(())
}

fn update_plugins<S>(name: Option<S>) -> Result<()>
where
    S: AsRef<str>,
    PathBuf: std::convert::From<S>,
{
    match name {
        Some(name) => {
            update_plugin(name)?;
            println!("No plugins found");
        }
        None => {
            let plugins_dir = plugin_path();
            let mut plugin_dir = fs::read_dir(&plugins_dir)?.peekable();

            if fs::metadata(&plugins_dir).is_ok() || plugin_dir.peek().is_none() {
                println!("No plugins found");
                return Ok(());
            }

            for entry in fs::read_dir(plugins_dir)?.filter_map(Result::ok) {
                let path = entry.path();
                println!("Updated {:?}", path.file_name().unwrap_or_default())
            }
        }
    }

    Ok(())
}

fn update_plugin(path: impl Into<PathBuf>) -> Result<()> {
    let repo = open_repo(path.into())?;
    let fetch = repo.remote_at("main")?;
    let _ = fetch
        .connect(Direction::Fetch)?
        .prepare_fetch(Discard, GixOptions::default())?
        .receive(Discard, &IS_INTERRUPTED);
    Ok(())
}

impl ComtryaCommand for PluginCommands {
    fn execute(&self, _runtime: &Runtime) -> anyhow::Result<()> {
        // ensure the plugin directory exists
        let plugin_dir = plugin_path();
        if !plugin_dir.exists() {
            create_dir_all(&plugin_dir)?;
        }

        match &self.command {
            PluginSubCommands::Add { name } => Ok(add_plugin(name)?),
            PluginSubCommands::List => Ok(list_plugins()?),
            PluginSubCommands::Remove { name } => Ok(remove_plugin(name)?),
            PluginSubCommands::Update { name } => Ok(update_plugins(name.as_ref())?),
        }
    }
}
