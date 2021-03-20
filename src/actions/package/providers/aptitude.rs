use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use crate::utils::command::{run_command, Command};
use serde::{Deserialize, Serialize};
use tracing::{debug, span, warn};
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

        run_command(Command {
            name: String::from("apt"),
            args: vec![
                String::from("install"),
                String::from("-y"),
                String::from("software-properties-common"),
                String::from("gpg"),
            ],
            require_root: true,
        })?;

        span.exit();

        Ok(())
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, package: &PackageVariant) -> Result<(), ActionError> {
        run_command(Command {
            name: String::from("apt-add-repository"),
            args: vec![String::from("-y"), package.repository.clone().unwrap()],
            require_root: true,
        })?;

        debug!(message = "Running Aptitude Update");

        run_command(Command {
            name: String::from("apt"),
            args: vec![String::from("update")],
            require_root: true,
        })?;

        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        run_command(Command {
            name: String::from("apt"),
            args: vec![String::from("install"), String::from("-y")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            require_root: true,
        })?;

        Ok(())
    }
}
