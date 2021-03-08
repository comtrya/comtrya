use super::PackageProvider;
use crate::actions::ActionError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    process::{Command, Stdio},
};
use tracing::info;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Homebrew {}

impl PackageProvider for Homebrew {
    fn available(&self) -> bool {
        match which("brew") {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        let client = Client::new();
        match client
            .get("https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh")
            .send()
        {
            Ok(mut res) => {
                let mut file = File::create("/tmp/brew-install.sh").unwrap();
                ::std::io::copy(&mut res, &mut file).unwrap();
            }
            Err(e) => {
                return Err(ActionError {
                    message: e.to_string(),
                });
            }
        };

        // Homebrew can only be used on Linux and macOS, so we can assume
        // we have access to bash ... right? 😅
        Command::new("bash")
            .args(&["/tmp/brew-install.sh"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        info!(message = "Installed Brew");

        Ok(())
    }

    fn has_repository(&self, _repository: &String) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, repository: &String) -> Result<(), ActionError> {
        match Command::new("brew").arg("tap").arg(repository).output() {
            Ok(_) => {
                info!(
                    message = "Added Package Repository",
                    repository = repository.as_str()
                );

                Ok(())
            }
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }

    fn install(&self, packages: Vec<String>) -> Result<(), ActionError> {
        match Command::new("brew").arg("install").args(&packages).output() {
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
