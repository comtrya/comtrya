use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use crate::utils::command::{run_command, Command};
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
        run_command(Command {
            name: String::from("pacman"),
            env: self.env(),
            args: vec![
                String::from("-S"),
                String::from("--noconfirm"),
                String::from("base-devel"),
                String::from("git"),
            ],
            require_root: true,
        })?;

        // Clone Yay from AUR
        run_command(Command {
            name: String::from("git"),
            env: self.env(),
            args: vec![
                String::from("clone"),
                String::from("https://aur.archlinux.org/yay.git"),
                String::from("/tmp/yay"),
            ],
            require_root: false,
        })?;

        // Install Yay from PKGBUILD
        run_command(Command {
            name: String::from("makepkg"),
            env: self.env(),
            args: vec![String::from("-si"), String::from("--noconfirm")],
            dir: String::from("/tmp/yay"),
            require_root: true,
        })?;

        span.exit();

        Ok(())
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
        run_command(Command {
            name: String::from("yay"),
            env: self.env(),
            args: vec![String::from("-S"), String::from("--noconfirm")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            require_root: false,
        })?;

        Ok(())
    }
}
