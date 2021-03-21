use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use serde::{Deserialize, Serialize};
use std::process::{Command, Output, Stdio};
use tracing::{debug, info, span, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Yay {}

impl PackageProvider for Yay {
    fn name(&self) -> &str {
        "Yay"
    }

    fn available(&self) -> bool {
        match which("yay") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "yay not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        let span = span!(tracing::Level::INFO, "bootstrap").entered();

        // Install base-devel and git to be able to pull and build/compile stuff
        Command::new("pacman")
            .args(&["-S", "--noconfirm", "base-devel", "git"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        // Clone Yay from AUR
        Command::new("git")
            .args(&["clone", "https://aur.archlinux.org/yay.git", "/tmp/yay"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        // Install Yay from PKGBUILD
        let result = match Command::new("makepkg")
            .args(&["-si", "--noconfirm"])
            .current_dir("/tmp/yay")
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
        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        match Command::new("yay")
            .args(&["-S", "--noconfirm"])
            .args(package.extra_args.clone())
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
