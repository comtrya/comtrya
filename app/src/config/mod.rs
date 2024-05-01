use crate::GlobalArgs;
use anyhow::Result;
use comtrya_lib::config::load_config as lib_config;
pub use comtrya_lib::config::Config;
use tracing::instrument;

#[instrument(name = "load_config", level = "info")]
pub(crate) fn load_config(args: &GlobalArgs) -> Result<Config> {
    match lib_config() {
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
