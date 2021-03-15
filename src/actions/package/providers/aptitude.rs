use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use serde::{Deserialize, Serialize};
use std::process::{Command, Output, Stdio};
use tracing::{debug, info, span, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aptitude {}

impl PackageProvider for Aptitude {
    fn name(&self) -> &str {
        "Aptitude"
    }

    fn available(&self) -> bool {
        match which("apt-add-repository") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "apt-add-repository not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        // Apt should always be available on Debian / Ubuntu flavours.
        // Lets make sure software-properties-common is available
        // for repository management
        let span = span!(tracing::Level::INFO, "bootstrap").entered();

        let result = match Command::new("apt")
            .args(&["install", "-y", "software-properties-common", "gpg"])
            .output()
        {
            Ok(Output { status, .. }) if status.success() => Ok(()),

            Ok(Output { stderr, .. }) => Err(ActionError {
                message: String::from_utf8(stderr).unwrap(),
            }),

            Err(e) => Err(ActionError {
                message: e.to_string(),
            }),
        };

        span.exit();

        result
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, package: &PackageVariant) -> Result<(), ActionError> {
        match Command::new("apt-add-repository")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .arg("-y")
            .arg(package.repository.clone().unwrap())
            .output()
        {
            Ok(_) => {
                debug!(message = "Apt Added Repository", repository = ?package.repository.clone().unwrap());
            }
            Err(error) => {
                return Err(ActionError {
                    message: error.to_string(),
                });
            }
        }

        debug!(message = "Running Aptitude Update");

        Command::new("apt")
            .arg("update")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        match Command::new("apt")
            .args(&["install", "-y"])
            .args(&package.packages())
            .output()
        {
            Ok(_) => {
                info!(
                    message = "Package Installed",
                    packages = package.packages().join(",").as_str()
                );

                Ok(())
            }
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }
}
