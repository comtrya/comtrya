use super::PackageProvider;
use crate::actions::ActionError;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use tracing::{debug, info};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aptitude {}

impl PackageProvider for Aptitude {
    fn available(&self) -> bool {
        match which("apt-add-repository") {
            Ok(_) => {
                info!(message = "apt-add-repository not available");
                true
            }
            Err(_) => false,
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        // Apt should always be available on Debian / Ubuntu flavours.
        // Lets make sure software-properties-common is available
        // for repository management
        info!(message = "Installing prerequisites for Aptitude package provider");

        Command::new("apt")
            .args(&["install", "-y", "software-properties-common", "gpg"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        Ok(())
    }

    fn has_repository(&self, _repository: &str) -> bool {
        false
    }

    fn add_repository(&self, repository: &str) -> Result<(), ActionError> {
        match Command::new("apt-add-repository")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .arg("-y")
            .arg(repository)
            .output()
        {
            Ok(_) => {
                debug!(message = "Apt Added Repository", repository = repository);
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

    fn install(&self, packages: Vec<String>) -> Result<(), ActionError> {
        match Command::new("apt")
            .args(&["install", "-y"])
            .args(&packages)
            .output()
        {
            Ok(_) => {
                info!(
                    message = "Package Installed",
                    packages = packages.clone().join(",").as_str()
                );

                Ok(())
            }
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }
}
