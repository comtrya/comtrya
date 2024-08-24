use std::io;

use commands::ComtryaCommand;

use clap::{Parser, Subcommand};
use comtrya_lib::contexts::build_contexts;
use comtrya_lib::contexts::Contexts;
use comtrya_lib::manifests;

use tracing::{error, Level};

#[allow(unused_imports)]
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, FmtSubscriber};

mod commands;
mod config;

use config::{load_config, Config};
#[derive(Parser, Debug)]
#[command(version, about, name="comtrya", long_about = None)]
struct GlobalArgs {
    #[arg(short = 'd', long)]
    pub manifest_directory: Option<String>,

    /// Disable color printing
    #[arg(long)]
    pub no_color: bool,

    /// Debug & tracing mode (-v, -vv)
    #[arg(short, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Apply manifests
    #[clap(aliases = &["do", "run"])]
    Apply(commands::Apply),

    /// Print version information
    Version(commands::Version),

    /// List available contexts
    Contexts(commands::Contexts),

    /// Auto generate completions
    ///
    /// for examples:
    ///  - bash: ```source <(comtrya gen-completions bash)```
    ///  - fish: ```comtrya gen-completions fish | source```
    #[command(long_about, verbatim_doc_comment)]
    GenCompletions(commands::GenCompletions),
}

#[derive(Debug)]
pub struct Runtime {
    pub(crate) args: GlobalArgs,
    pub(crate) config: Config,
    pub(crate) contexts: Contexts,
}

pub(crate) fn execute(runtime: Runtime) -> anyhow::Result<()> {
    match &runtime.args.command {
        Commands::Apply(apply) => apply.execute(&runtime),
        Commands::Version(version) => version.execute(&runtime),
        Commands::Contexts(contexts) => contexts.execute(&runtime),
        Commands::GenCompletions(gen_completions) => gen_completions.execute(&runtime),
    }
}

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

    let config = match load_config(&args) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", error.to_string());
            panic!();
        }
    };

    if !config.disable_update_check {
        check_for_updates(args.no_color);
    }

    // Run Context Providers
    let contexts = build_contexts(&config);

    let runtime = Runtime {
        args,
        config,
        contexts,
    };

    execute(runtime)?;

    Ok(())
}

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

        let cmd = format!(
            "Run to update: {cmd}",
            cmd = "curl -fsSL https://get.comtrya.dev | sh".green()
        );

        println!("\n{msg}\n{changelog}\n{cmd}");
    }
}
