use super::PackageProvider;
use crate::actions::package::PackageVariant;
use crate::utils::command::{run_command, Command};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{instrument, span, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BsdPkg {}

impl BsdPkg {
    fn env(&self) -> HashMap<String, String> {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert(String::from("ASSUME_ALWAYS_YES"), String::from("true"));
        env
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

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self) -> Result<()> {
        let span = span!(tracing::Level::INFO, "bootstrap").entered();

        run_command(Command {
            name: String::from("/usr/sbin/pkg"),
            env: self.env(),
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

    fn add_repository(&self, _package: &PackageVariant) -> Result<()> {
        // I don't know what this looks like yet
        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages for now, don't get smart about which
        // already are
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<()> {
        // Manually ensure we have sudo
        if "root" != whoami::username() {
            match std::process::Command::new("sudo")
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .arg("--validate")
                .output()
            {
                Ok(std::process::Output { status, .. }) if status.success() => (),

                _ => return Err(anyhow!("Failed to get sudo access")),
            };
        }

        let mut command = if "root" == whoami::username() {
            std::process::Command::new("sudo")
        } else {
            std::process::Command::new("pkg")
        };

        if "root" == whoami::username() {
            command.arg("pkg");
        }

        let result = command
            .envs(self.env())
            .args(
                vec![String::from("install"), String::from("-n")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages()),
            )
            .output();

        // Rerun without dry-run / -n if nothing is to be removed
        match result {
            Ok(std::process::Output { status, stdout, .. }) if status.success() => {
                // Command run OK, check for removed
                let out_string = String::from_utf8(stdout).unwrap();
                if out_string.to_lowercase().contains("removed") {
                    return Err(anyhow!(format!(
                        "Installing '{}' would remove packages. Please run this manually",
                        package.packages().join(",")
                    )));
                }
            }
            Ok(std::process::Output { .. }) => {
                return Err(anyhow!("Failed to install packages"));
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
        }

        // OK to install
        let result = command
            .envs(self.env())
            .args(
                vec![String::from("install")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages()),
            )
            .output();

        match result {
            Ok(std::process::Output { status, .. }) if status.success() => Ok(()),
            Ok(std::process::Output { .. }) => Err(anyhow!("Failed to install packages")),
            Err(e) => Err(anyhow!(e)),
        }
    }
}
