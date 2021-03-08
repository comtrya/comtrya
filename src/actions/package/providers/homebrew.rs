use super::PackageProvider;
use crate::actions::ActionError;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    ops::Deref,
    process::{Command, ExitStatus, Output, Stdio},
};
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
        let installer = Command::new("bash")
            .args(&["/tmp/brew-install.sh"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        println!("Brew install {:?}", String::from_utf8(installer.stdout));

        Ok(())
    }

    fn install(&self, packages: Vec<String>) -> Result<(), ActionError> {
        match Command::new("brew").arg("install").args(packages).output() {
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
