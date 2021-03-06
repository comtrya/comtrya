use serde::{Deserialize, Serialize};

use super::Package;

pub mod apt;
pub mod homebrew;

#[derive(Debug, Clone)]
pub struct PackageProviderError;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageProviders {
    Apt,
    Homebrew,
    Scoop,
}

pub trait PackageProvider {
    /// Check that the provider supports this OS
    fn supported(&self) -> bool;
    /// Checks that the provider command is available, installing it if it isn't.
    fn init(&self) -> Result<bool, PackageProviderError>;
    fn add_repository(&self) -> Result<bool, PackageProviderError>;
    fn install(&self, package: &Package) -> Result<bool, PackageProviderError>;
    fn upgrade(&self) -> Result<bool, PackageProviderError>;
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::Debian => PackageProviders::Apt,
            os_info::Type::Macos => PackageProviders::Homebrew,
            os_info::Type::Mint => PackageProviders::Apt,
            os_info::Type::Pop => PackageProviders::Apt,
            os_info::Type::Ubuntu => PackageProviders::Apt,
            os_info::Type::Windows => PackageProviders::Scoop,
            os_info::Type::OracleLinux => PackageProviders::Apt,

            _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}
