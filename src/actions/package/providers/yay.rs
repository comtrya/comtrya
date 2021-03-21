use std::collections::HashMap;

use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use crate::utils::command::{run_command, Command};
use serde::{Deserialize, Serialize};
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
        info!(message = "Installing base-devel and git");
        run_command(Command {
            name: String::from("pacman"),
            env: HashMap::new(),
            dir: None,
            args: vec![
                String::from("-S"),
                String::from("--noconfirm"),
                String::from("base-devel"),
                String::from("git"),
            ],
            require_root: true,
        })?;

        // Clone Yay from AUR
        info!(message = "Cloning yay's PKGBUILD");
        run_command(Command {
            name: String::from("git"),
            env: HashMap::new(),
            dir: None,
            args: vec![
                String::from("clone"),
                String::from("https://aur.archlinux.org/yay.git"),
                String::from("/tmp/yay"),
            ],
            require_root: false,
        })?;

        // Install Yay from PKGBUILD
        info!(message = "Building and Installing yay using PKGBUILD script");
        run_command(Command {
            name: String::from("makepkg"),
            env: HashMap::new(),
            dir: Some(String::from("/tmp/yay")),
            args: vec![String::from("-si"), String::from("--noconfirm")],
            require_root: false,
        })?;

        // Clean up
        info!(message = "Cleaning up temporary PKGBUILD folder");
        run_command(Command {
            name: String::from("rm"),
            env: HashMap::new(),
            dir: None,
            args: vec![String::from("-rf"), String::from("/tmp/yay")],
            require_root: false,
        })?;

        info!(message = "Yay installed");

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
            env: HashMap::new(),
            dir: None,
            args: vec![
                String::from("-S"),
                String::from("--noconfirm"),
                String::from("--nocleanmenu"),
                String::from("--nodiffmenu"),
            ]
            .into_iter()
            .chain(package.extra_args.clone())
            .chain(package.packages())
            .collect(),
            require_root: false,
        })?;

        Ok(())
    }
}
