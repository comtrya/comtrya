use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use crate::utils::command::{run_command, Command};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{span, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BsdPkg {}

impl BsdPkg {
    fn env(&self) -> HashMap<String, String> {
        HashMap::new()
    }
}

impl PackageProvider for BsdPkg {
    fn name(&self) -> &str {
        "BsdPkg"
    }

    fn available(&self) -> bool {
        match which("/usr/local/sbin/pkg") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "/usr/local/sbin/pkg is not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        let span = span!(tracing::Level::INFO, "bootstrap").entered();

        let mut env = HashMap::new();
        env.insert(String::from("ALWAYS_ASSUME_YES"), String::from("1"));

        run_command(Command {
            name: String::from("/usr/local/sbin/pkg"),
            env,
            dir: None,
            args: vec![String::from("bootstrap")],
            require_root: true,
        })?;

        span.exit();

        Ok(())
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, _package: &PackageVariant) -> Result<(), ActionError> {
        // I don't know what this looks like yet
        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages for now, don't get smart about which
        // already are
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        run_command(Command {
            name: String::from("pkg"),
            env: self.env(),
            dir: None,
            // This -n will stop the installation from removing any packages
            args: vec![String::from("install"), String::from("-n")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            require_root: true,
        })?;

        Ok(())
    }
}
