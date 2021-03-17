use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, trace, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Winget {}

impl PackageProvider for Winget {
    fn name(&self) -> &str {
        "Winget"
    }

    fn available(&self) -> bool {
        match which("winget") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "winget not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        // Not sure if we can automate this atm, we'll require it
        // be installed upfront for the time being
        Err(ActionError {
            message: String::from("Winget is not available. Please install"),
        })
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        true
    }

    fn add_repository(&self, _package: &PackageVariant) -> Result<(), ActionError> {
        Ok(())
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages, make this smarter soon
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        let result = package
            .packages()
            .into_iter()
            .try_fold(vec![], |mut acc, p| {
                match Command::new("winget")
                    .args(&["install", "--silent"])
                    .args(package.extra_args.clone())
                    .arg(&p)
                    .output()
                {
                    Ok(result) => {
                        debug!("Installed {}", p.clone());

                        acc.push(p);

                        trace!("{:?}", String::from_utf8(result.stdout).unwrap());

                        Ok(acc)
                    }
                    Err(error) => {
                        debug!("Failed to install {}", p);
                        trace!("{:?}", error.to_string());

                        Err(ActionError {
                            message: String::from(format!(
                                "Failed to install {}, but successfully installed {:?}",
                                p,
                                acc.join(",")
                            )),
                        })
                    }
                }
            });

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
