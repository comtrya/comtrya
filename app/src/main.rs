//! `comtrya`: applies manifests to the system it runs on

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

use crate::commands::ComtryaCommand;
use crate::config::{Commands, GlobalArgs};

use std::io;

use comtrya_lib::contexts::{Contexts, Facts};
use comtrya_lib::manifests;

use clap::Parser;
use tracing::{error, Level};

use anyhow::Context;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;
use tracing_subscriber::{fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, Layer};

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
        Commands::Helper => unreachable!("the helper runs before the runtime is built"),
    }
}

/// Logs to stdout, more with each `-v`, to `--log-file` if given, and to journald on Linux when
/// it's there
fn configure_tracing(args: &GlobalArgs) -> anyhow::Result<()> {
    let stdout = fmt::layer()
        .with_ansi(!args.no_color)
        .with_target(false)
        .without_time()
        .with_writer(io::stdout.with_max_level(level(args.verbose)));

    let file = match &args.log_file {
        Some(path) => Some(file_layer(path, args.verbose)?),
        None => None,
    };

    let subscriber = tracing_subscriber::registry().with(stdout).with(file);

    #[cfg(target_os = "linux")]
    let subscriber = subscriber.with(tracing_journald::layer().ok());

    tracing::subscriber::set_global_default(subscriber).expect("Unable to set a global subscriber");
    Ok(())
}

/// What stdout shows: info, more with each `-v`
fn level(verbose: u8) -> Level {
    match verbose {
        0 => Level::INFO,
        1 => Level::DEBUG,
        _ => Level::TRACE,
    }
}

/// Writes the log to the file at `path`, appending. It gets at least what `-v` shows, because a
/// log file is read later, when it's too late to run again with `-v`
fn file_layer<S>(path: &Path, verbose: u8) -> anyhow::Result<impl Layer<S>>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("can't open the log file {}", path.display()))?;

    Ok(fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .fmt_fields(PlainFields::default())
        .with_writer(Mutex::new(file).with_max_level(level(verbose.max(1)))))
}

/// Formats fields like stdout does. A type of its own, because the fields of a span are formatted
/// once per formatter type: with stdout's type, the file would get stdout's colors
#[derive(Default)]
struct PlainFields(fmt::format::DefaultFields);

impl<'writer> fmt::FormatFields<'writer> for PlainFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: fmt::format::Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        self.0.format_fields(writer, fields)
    }
}

fn main() -> anyhow::Result<()> {
    let args = GlobalArgs::parse();
    if let Commands::Helper = args.command {
        // It answers on stdout, so nothing else may write there, not even the log. It reads no
        // configuration and gathers no facts: it runs as root and only does what it's sent
        return Ok(comtrya_lib::helper::serve(
            io::stdin().lock(),
            io::stdout().lock(),
        )?);
    }
    if let Err(error) = configure_tracing(&args) {
        eprintln!("{error:#}");
        std::process::exit(1);
    }

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
