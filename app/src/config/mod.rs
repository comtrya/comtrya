use crate::GlobalArgs;
use anyhow::Result;
use comtrya_lib::config::load_config as lib_config;
pub use comtrya_lib::config::Config;
use tracing::instrument;

#[instrument(name = "load_config", level = "info")]
pub(crate) fn load_config(args: GlobalArgs) -> Result<Config> {
    let config = lib_config()?;

    let plugins_directory = args.plugins_directory.unwrap_or(config.plugins_path);
    let manifest_directory = args
        .manifest_directory
        .map(|dir| vec![dir])
        .unwrap_or(config.manifest_paths);

    Ok(Config {
        plugins_path: plugins_directory,
        manifest_paths: manifest_directory,
        ..config
    })
}
