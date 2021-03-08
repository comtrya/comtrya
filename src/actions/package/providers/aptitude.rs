use super::PackageProvider;
use crate::actions::ActionError;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aptitude {}

impl PackageProvider for Aptitude {
    fn available(&self) -> bool {
        match which("apt-add-repository") {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        // Apt should always be available on Debian / Ubuntu flavours.
        // Lets make sure software-properties-common is available
        // for repository management
        let installer = Command::new("apt")
            .args(&["install", "-y", "software-properties-common", "gpg"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        println!("Apt install {:?}", String::from_utf8(installer.stdout));

        Ok(())
    }

    fn has_repository(&self, _repository: &String) -> bool {
        false
    }

    fn add_repository(&self, repository: &String) -> Result<(), ActionError> {
        match Command::new("apt-add-repository")
            .env("DEBIAN_FRONTEND", "noninteractive")
            .arg("-y")
            .arg(repository)
            .output()
        {
            Ok(o) => {
                println!(
                    "Added repository {:?}: {:?} output: {:?} and {:?}",
                    repository,
                    o.status,
                    String::from_utf8(o.stdout),
                    String::from_utf8(o.stderr)
                );

                ()
            }
            Err(error) => {
                return Err(ActionError {
                    message: error.to_string(),
                });
            }
        }

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
            .args(packages)
            .output()
        {
            Ok(o) => {
                println!(
                    "Installed {:?} output: {:?} and {:?}",
                    o.status,
                    String::from_utf8(o.stdout),
                    String::from_utf8(o.stderr)
                );

                Ok(())
            }
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }
}
