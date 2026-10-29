//! `comtrya`: applies manifests to the system it runs on

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use crate::commands::ComtryaCommand;
use crate::config::{Commands, GlobalArgs};

use std::io;

use comtrya_lib::contexts::{Contexts, Facts};
use comtrya_lib::manifests;

use clap::Parser;
use tracing::{error, Level};

#[allow(unused_imports)]
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, FmtSubscriber};

/// The subcommands
mod commands;
/// Command line arguments and `Comtrya.yaml`
mod config;
use config::Config;

/// What every subcommand gets: the arguments, the configuration and the contexts
#[derive(Debug)]
pub struct Runtime {
    /// The command line arguments
    pub(crate) args: GlobalArgs,
    /// `Comtrya.yaml`, with the overrides from the command line
    pub(crate) config: Config,
    /// The typed facts about the system and the configuration
    pub(crate) facts: Facts,
    /// The same facts, for templates and `where` conditions
    pub(crate) contexts: Contexts,
}

/// Runs the subcommand from the arguments
pub(crate) fn execute(runtime: Runtime) -> anyhow::Result<()> {
    match &runtime.args.command {
        Commands::Apply(apply) => apply.execute(&runtime),
        Commands::Status(apply) => apply.status(&runtime),
        Commands::Version(version) => version.execute(&runtime),
        Commands::Contexts(contexts) => contexts.execute(&runtime),
        Commands::GenCompletions(gen_completions) => gen_completions.execute(&runtime),
    }
}

/// Logs to stdout, more with each `-v`, and to journald on Linux when it's there
fn configure_tracing(args: &GlobalArgs) {
    let stdout_writer = match args.verbose {
        0 => io::stdout.with_max_level(tracing::Level::INFO),
        1 => io::stdout.with_max_level(tracing::Level::DEBUG),
        _ => io::stdout.with_max_level(tracing::Level::TRACE),
    };

    let builder = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_ansi(!args.no_color)
        .with_target(false)
        .with_writer(stdout_writer)
        .without_time();

    #[cfg(target_os = "linux")]
    if let Ok(layer) = tracing_journald::layer() {
        tracing::subscriber::set_global_default(builder.finish().with(layer))
            .expect("Unable to set a global subscriber");
        return;
    }

    tracing::subscriber::set_global_default(builder.finish())
        .expect("Unable to set a global subscriber");
}

fn main() -> anyhow::Result<()> {
    let args = GlobalArgs::parse();
    configure_tracing(&args);

    let config = match config::load_config(&args) {
        Ok(config) => config,
        Err(error) => {
            error!("{error:#}");
            std::process::exit(1);
        }
    };

    if !config.disable_update_check {
        check_for_updates(args.no_color);
    }

    // Run Context Providers
    let facts = Facts::gather(&config);
    let contexts = facts.to_contexts();
    let runtime = Runtime {
        args,
        config,
        facts,
        contexts,
    };

    execute(runtime)?;

    Ok(())
}

/// Tells the user about a newer comtrya on crates.io
fn check_for_updates(no_color: bool) {
    use colored::*;
    use update_informer::{registry, Check};

    if no_color {
        control::set_override(false);
    }

    let pkg_name = env!("CARGO_PKG_NAME");
    let pkg_version = env!("CARGO_PKG_VERSION");
    let informer = update_informer::new(registry::Crates, pkg_name, pkg_version);

    if let Some(new_version) = informer.check_version().ok().flatten() {
        let msg = format!(
            "A new version of {pkg_name} is available: v{pkg_version} -> {new_version}",
            pkg_name = pkg_name.italic().cyan(),
            new_version = new_version.to_string().green()
        );

        let release_url =
            format!("https://github.com/{pkg_name}/{pkg_name}/releases/tag/{new_version}").blue();
        let changelog = format!("Changelog: {release_url}",);

        let update = if cfg!(windows) {
            "irm https://comtrya.dev/install.ps1 | iex"
        } else {
            "curl -fsSL https://comtrya.dev/install.sh | sh"
        };
        let cmd = format!("Run to update: {cmd}", cmd = update.green());

        println!("\n{msg}\n{changelog}\n{cmd}");
    }
}
