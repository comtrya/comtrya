mod aptitude;
mod homebrew;
mod winget;
mod yay;

use self::aptitude::Aptitude;
use self::homebrew::Homebrew;
use self::winget::Winget;
use self::yay::Yay;
use crate::actions::ActionError;
use serde::{Deserialize, Serialize};

use super::PackageVariant;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(alias = "homebrew", alias = "brew")]
    Homebrew,
    #[serde(alias = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,
    #[serde(alias = "yay")]
    Yay,

    #[serde(alias = "winget")]
    Winget,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Homebrew => Box::new(Homebrew {}),
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::Winget => Box::new(Winget {}),
            PackageProviders::Yay => Box::new(Yay {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            // Debian / Ubuntu Variants
            os_info::Type::Debian => PackageProviders::Aptitude,
            os_info::Type::Mint => PackageProviders::Aptitude,
            os_info::Type::Pop => PackageProviders::Aptitude,
            os_info::Type::Ubuntu => PackageProviders::Aptitude,
            // For some reason, the Rust image is showing as this and
            // its Debian based?
            os_info::Type::OracleLinux => PackageProviders::Aptitude,
            os_info::Type::Macos => PackageProviders::Homebrew,
            os_info::Type::Windows => PackageProviders::Winget,
            // Arch Variants
            os_info::Type::Manjaro=> PackageProviders::Yay,
            os_info::Type::Arch=> PackageProviders::Yay,

            _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}

pub trait PackageProvider {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn bootstrap(&self) -> Result<(), ActionError>;
    fn has_repository(&self, package: &PackageVariant) -> bool;
    fn add_repository(&self, package: &PackageVariant) -> Result<(), ActionError>;
    fn query(&self, package: &PackageVariant) -> Vec<String>;
    fn install(&self, package: &PackageVariant) -> Result<(), ActionError>;
}
