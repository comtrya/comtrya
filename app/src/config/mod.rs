use crate::commands;
use clap::{Parser, Subcommand};

use anyhow::Context;
use anyhow::Result;
pub use comtrya_lib::config::Config;
use std::{
    path::{Path, PathBuf},
    vec,
    // fs::File,
    // io::Write,
};

// use tempfile::tempdir;
use tracing::error;
use tracing::{trace, warn};


#[derive(Parser, Debug)]
#[command(version, about, name="comtrya", long_about = None)]
pub struct GlobalArgs {
    #[arg(short = 'd', long)]
    pub manifest_directory: Option<String>,

    #[arg(short = 'c', long)]
    pub config_path: Option<String>,

    /// Disable color printing
    #[arg(long)]
    pub no_color: bool,

    /// Debug & tracing mode (-v, -vv)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Apply manifests
    #[clap(aliases = &["do", "run"])]
    Apply(commands::Apply),

    ///  List manifests status (ALPHA)
    Status(commands::Apply),

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

pub(crate) fn load_config(args: &GlobalArgs) -> Result<Config> {
    match lib_config(&args) {
        Ok(config) => match args.manifest_directory.clone() {
            Some(manifest_path) => Ok(Config {
                manifest_paths: vec![manifest_path],
                ..config
            }),
            None => Ok(Config { ..config }),
        },
        Err(error) => Err(error),
    }
}

/// Check the current working directory for a `Comtrya.yaml` file
/// If that doesn't exist, we'll check the platforms config directory
/// for comtrya/Comtrya.yaml
///
/// # Arguments
///
/// * `args` - A reference to a `GlobalArgs` struct containing user-supplied arguments
///
/// # Returns
///
/// `Result<Config>`
/// - `Ok(Config)` - valid `Comtrya.yaml` file is found and deserialized successfully
/// - `Err` - Error occurs during reading/deserialization OR a user provided an invalid
///     path for the config file
///
/// # Errors
///
/// Exits if the user specified an invalid config file path
/// returns errors if file read fails or yaml content is not successfully deserialized
pub fn lib_config(args: &GlobalArgs) -> Result<Config> {
    let config = match find_configs(&args) {
        Some(config_path) => {
            let yaml = std::fs::read_to_string(&config_path)
                .with_context(|| "Found Comtrya.yaml, but was unable to read the contents.")?;

            let mut config = match yaml.trim().is_empty() {
                true => Config {
                    ..Default::default()
                },

                false => serde_yml::from_str(yaml.as_str())
                    .with_context(|| "Found Comtrya.yaml, but couldn't deserialize the YAML.")?,
            };

            // The existence of the config file allows an implicit manifests location of.
            if config.manifest_paths.is_empty() {
                if let Some(parent) = config_path.parent() {
                    config.manifest_paths.push(parent.display().to_string());
                }
            }

            config
        }

        None => {
            // Panic early if an incorrect configuration path was specified by the user.
            if let Some(cfg_path) = &args.config_path {
                error!("The user specified a file at {} but none exists.", cfg_path);
                std::process::exit(1);
            }

            Config {
                manifest_paths: vec![String::from(",")],
                ..Default::default()
            }
        }
    };

    Ok(config)
}

/// Attempts to find the configuration file, either based on optional user input
/// or the default working directory/platform's config directory
///
/// # Arguments
///
/// * `args` - A reference to a `GlobalArgs` struct containing user-supplied arguments, including
///           an optional `config_path` field that may specify a configuration file location.
///
/// # Returns
///
/// `Option<PathBuf>`:
/// - `Some(PathBuf)` if a valid configuration file is found at the provided path,
///   the current working directory, or the system's configuration directory.
/// - `None` if no valid configuration file is found.
///
/// # Errors
///
/// Returns `None` if:
/// - User-provided path is invalid
/// - Error occurs while checking the user-provided path
/// - No config file is found in the working directory or platform's config directory
fn find_configs(args: &GlobalArgs) -> Option<PathBuf> {
    // Check if the user specified a configuration path
    if let Some(cfg_path) = &args.config_path {
        let cfg_path = Path::new(cfg_path);

        // If the user specified a path that is not found, we should fail early and not fall back
        // to the default path.
        match cfg_path.try_exists() {
            Ok(true) => return Some(cfg_path.into()),
            Ok(false) => {
                trace!("Specified path does not exist.");
                return None;
            }
            Err(e) => {
                trace!("Error checking path existence: {}", e);
                return None;
            }
        }
    }

    // Check current working directory first
    if let Ok(cwd) = std::env::current_dir() {
        let local_config = cwd.join("Comtrya.yaml");

        if local_config.is_file() {
            warn!("Comtrya.yaml found in current working directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in current working directory");
    }

    // Check platform's config dir
    if let Some(config_dir) = dirs_next::config_dir() {
        let local_config = config_dir.join("Comtrya.yaml");

        if local_config.is_file() {
            warn!("Comtrya.yaml found in users config directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in users config directory");
    };

    None
}

// fn create_temp_config(content: &str) -> std::path::PathBuf {
//     let temp_dir = tempdir().unwrap();
//     let config_path = temp_dir.path().join("Comtrya.yaml");
//     let mut file = File::create(&config_path).unwrap();
//     file.write_all(content.as_bytes()).unwrap();
//     config_path
// }
//
// #[test]
// fn test_lib_config_with_valid_config_path() {
//     let valid_config_path = create_temp_config("some: value\n");
//     let args = GlobalArgs {
//         config_path: Some(valid_config_path.to_string_lossy().to_string()),
//         ..Default::default()
//     };
// }
