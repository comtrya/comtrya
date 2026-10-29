//! The command line arguments, and how they and `Comtrya.yaml` make the configuration

use crate::commands;
use clap::{Parser, Subcommand};
use std::error::Error;

use anyhow::{anyhow, Context, Result};
pub use comtrya_lib::config::Config;
use std::{
    path::{Path, PathBuf},
    vec,
};

use tracing::{debug, trace};

/// The arguments of `comtrya`
#[derive(Parser, Debug, Default)]
#[command(version, about, name="comtrya", long_about = None)]
pub struct GlobalArgs {
    /// Directory or git repository of the manifests, instead of `manifest_paths` from the config
    #[arg(short = 'd', long)]
    pub manifest_directory: Option<String>,

    /// Specify a configuration path (if invalid Comtrya will exit)
    #[arg(short = 'c', long)]
    pub config_path: Option<String>,

    /// Disable color printing
    #[arg(long)]
    pub no_color: bool,

    /// Set a variable, as `key=value`. Wins over `variables` from the config
    #[arg(short = 'D', long, value_parser = parse_key_val::<String, String>)]
    pub defines: Vec<(String, String)>,

    /// Debug & tracing mode (-v, -vv)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// The subcommand
    #[command(subcommand)]
    pub command: Commands,
}

/// The subcommands
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

impl Default for Commands {
    fn default() -> Self {
        Commands::Version(commands::Version {})
    }
}

/// Parse a single key-value pair
fn parse_key_val<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no `=` found in `{s}`"))?;
    Ok((s[..pos].parse()?, s[pos + 1..].parse()?))
}

/// The configuration: `Comtrya.yaml`, with the overrides from the command line
pub(crate) fn load_config(args: &GlobalArgs) -> Result<Config> {
    match lib_config(args) {
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
/// for Comtrya.yaml. Relative `manifest_paths` are relative to the directory of the file
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
///   path for the config file
///
/// # Errors
///
/// Exits if the user specified an invalid config file path
/// returns errors if file read fails or yaml content is not successfully deserialized
pub fn lib_config(args: &GlobalArgs) -> anyhow::Result<Config> {
    let mut config = match find_configs(args) {
        Some(config_path) => {
            let yaml = std::fs::read_to_string(&config_path)
                .with_context(|| "Found Comtrya.yaml, but was unable to read the contents.")?;

            let mut config = match yaml.trim().is_empty() {
                true => Config {
                    ..Default::default()
                },

                false => serde_yaml_ng::from_str(yaml.as_str())
                    .with_context(|| "Found Comtrya.yaml, but couldn't deserialize the YAML.")?,
            };

            // Without `manifest_paths`, the manifests are next to the config file
            let dir = config_dir(&config_path);
            for path in &mut config.manifest_paths {
                *path = relative_to(&dir, path);
            }
            if config.manifest_paths.is_empty() {
                config.manifest_paths.push(dir);
            }

            config
        }

        None => {
            // Panic early if an incorrect configuration path was specified by the user.
            if let Some(cfg_path) = &args.config_path {
                return Err(anyhow!(
                    "The user specified a file at {cfg_path} but none exists."
                ));
            }

            Config {
                manifest_paths: vec![String::from(".")],
                ..Default::default()
            }
        }
    };

    let defines_iterator = args.defines.iter();
    for pair in defines_iterator {
        config.variables.insert(pair.0.clone(), pair.1.clone());
    }

    Ok(config)
}

/// The directory of the config file at `config_path`: `.` for a bare file name
fn config_dir(config_path: &Path) -> String {
    match config_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.display().to_string(),
        _ => String::from("."),
    }
}

/// An entry of `manifest_paths` in the config file in `dir`: a relative directory is relative to
/// `dir`, URLs and absolute directories stay as they are
fn relative_to(dir: &str, path: &str) -> String {
    if path.contains("://") || Path::new(path).is_absolute() {
        return path.to_string();
    }
    Path::new(dir).join(path).display().to_string()
}

/// Attempts to find the configuration file, either based on optional user input
/// or the default working directory/platform's config directory
///
/// # Arguments
///
/// * `args` - A reference to a `GlobalArgs` struct containing user-supplied arguments, including
///   an optional `config_path` field that may specify a configuration file location.
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
            debug!("Comtrya.yaml found in current working directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in current working directory");
    }

    // Check platform's config dir
    if let Some(config_dir) = dirs::config_dir() {
        let local_config = config_dir.join("Comtrya.yaml");

        if local_config.is_file() {
            debug!("Comtrya.yaml found in users config directory");
            return Some(local_config);
        }
        trace!("No Comtrya.yaml found in users config directory");
    };

    None
}

#[cfg(test)]
mod tests {
    use crate::config::{config_dir, lib_config, GlobalArgs};
    use std::path::PathBuf;

    fn get_config_file() -> PathBuf {
        std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("Comtrya.yaml")
    }

    /// Test config returns an error when an invalid configuration path is supplied
    #[test]
    fn load_config_invalid_path() {
        let args = GlobalArgs {
            config_path: Some("/invalid/path/to/config.yaml".to_string()),
            ..Default::default()
        };

        let result = lib_config(&args);
        assert!(
            result.is_err(),
            "Expected an error when the user supplied config path is invalid"
        );
    }

    /// Test valid configuration path
    #[test]
    fn load_config_valid_path() {
        let args = GlobalArgs {
            config_path: Some(get_config_file().to_string_lossy().to_string()),
            ..Default::default()
        };

        let result = lib_config(&args);
        assert!(result.is_ok());
    }

    /// Writes `yaml` as `Comtrya.yaml` into a new directory, and loads it
    fn load(yaml: &str) -> (tempfile::TempDir, crate::config::Config) {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("Comtrya.yaml");
        std::fs::write(&config_path, yaml).unwrap();
        let args = GlobalArgs {
            config_path: Some(config_path.display().to_string()),
            ..Default::default()
        };

        let config = lib_config(&args).unwrap();
        (dir, config)
    }

    #[test]
    fn relative_manifest_paths_are_relative_to_the_config_file() {
        let (dir, config) = load("manifest_paths:\n  - manifests\n");

        assert_eq!(
            vec![dir.path().join("manifests").display().to_string()],
            config.manifest_paths
        );
    }

    #[test]
    fn absolute_and_remote_manifest_paths_are_kept() {
        let absolute = std::env::temp_dir().display().to_string();
        let remote = "https://github.com/comtrya/comtrya#main:examples";
        let (_dir, config) = load(&format!("manifest_paths:\n  - {absolute}\n  - {remote}\n"));

        assert_eq!(vec![absolute, remote.to_string()], config.manifest_paths);
    }

    #[test]
    fn a_config_file_without_a_directory_is_in_the_current_one() {
        assert_eq!(".", config_dir(std::path::Path::new("Comtrya.yaml")));
        assert_eq!(
            "dotfiles",
            config_dir(std::path::Path::new("dotfiles/Comtrya.yaml"))
        );
    }
}
