use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    path::Path,
    process::{Command, Output, Stdio},
};
use tracing::{debug, error, info, trace, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Homebrew {}

impl PackageProvider for Homebrew {
    fn name(&self) -> &str {
        "Homebrew"
    }

    fn available(&self) -> bool {
        which("brew").is_ok()
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

        info!(message = "Installed Homebrew");

        Ok(())
    }

    fn has_repository(&self, _: &PackageVariant) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, package: &PackageVariant) -> Result<(), ActionError> {
        let repository = package.repository.clone().unwrap();

        match Command::new("brew").arg("tap").arg(&repository).output() {
            Ok(_) => {
                info!(
                    message = "Added Package Repository",
                    repository = ?repository
                );

                Ok(())
            }
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        let prefix = String::from_utf8(
            Command::new("brew")
                .arg("--prefix")
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .replace("\n", "")
        .replace("\r", "");

        let cellar = Path::new(&prefix).join("Cellar");
        let caskroom = Path::new(&prefix).join("Caskroom");

        package
            .packages()
            .into_iter()
            .filter(|p| {
                if cellar.join(&p).is_dir() {
                    trace!("{}: found in Cellar", p);
                    false
                } else if caskroom.join(&p).is_dir() {
                    trace!("{}: found in Caskroom", p);
                    false
                } else {
                    debug!("{}: doesn't appear to be installed", p);
                    true
                }
            })
            .map(|p| match &package.repository {
                Some(repository) => format!("{}/{}", repository, p),
                None => p,
            })
            .collect()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        let need_installed = self.query(package);

        if need_installed.is_empty() {
            return Ok(());
        }

        debug!(
            "Installing with extra args: {}",
            package.extra_args.clone().join(",")
        );

        match Command::new("brew")
            .arg("install")
            .args(package.extra_args.clone())
            .args(&need_installed)
            .output()
        {
            Ok(Output { status, .. }) if status.success() => {
                info!(packages = need_installed.clone().join(",").as_str());
                Ok(())
            }

            Ok(Output { stdout, stderr, .. }) => {
                warn!("{}", String::from_utf8(stdout).unwrap().as_str());
                error!("{}", String::from_utf8(stderr).unwrap().as_str());

                Err(ActionError {
                    message: format!("Failed to install {}", need_installed.join(" ")),
                })
            }

            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }
}
