use crate::packages::providers::{PackageProvider, PackageProviderError, PackageProviders};
use std::process::Command;
use which::which;

pub struct Homebrew {}

impl Homebrew {
    fn supported(&self) -> bool {
        true
    }

    fn init(&self) -> Result<bool, super::PackageProviderError> {
        match which::which("brew") {
            Ok(_) => return Ok(false),
            Err(_) => (),
        };

        // Homebrew can only be used on Linux and macOS, so we can assume
        // we have access to bash and curl ... right? 😅
        match Command::new("bash")
            .args(&[
                "-c",
                "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)",
            ])
            .status()
        {
            Ok(_) => Ok(true),
            Err(_) => Err(PackageProviderError {}),
        }
    }

    fn add_repository(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }

    fn install(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }

    fn upgrade(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }
}
