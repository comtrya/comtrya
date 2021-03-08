use crate::packages::Package;
use std::process::Command;
use which::which;

use super::PackageProvider;

pub struct Aptitude {}

impl PackageProvider for Aptitude {
    fn supported(&self) -> bool {
        true
    }

    fn init(&self) -> Result<bool, super::PackageProviderError> {
        match which("apt") {
            Ok(_) => Ok(false),
            // Error, don't know how to install apt: provider unavailable
            Err(_) => Err(super::PackageProviderError {}),
        }
    }

    fn add_repository(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }

    fn install(&self, package: &Package) -> Result<bool, super::PackageProviderError> {
        Command::new("apt")
            .arg(format!("install -y {}", package.list.join(" ")))
            .output()
            .unwrap();

        println!("Installed {}", package.list.join(" "));

        Ok(true)
    }

    fn upgrade(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }
}
