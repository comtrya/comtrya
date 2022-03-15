mod aptitude;
use self::aptitude::Aptitude;
use crate::steps::Step;
mod bsdpkg;
use self::bsdpkg::BsdPkg;
mod homebrew;
use self::homebrew::Homebrew;
mod yay;
use self::yay::Yay;
mod winget;
use self::winget::Winget;
use super::PackageVariant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(alias = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,

    #[serde(alias = "bsdpkg")]
    BsdPkg,

    #[serde(alias = "homebrew", alias = "brew")]
    Homebrew,

    #[serde(alias = "yay", alias = "pacman")]
    Yay,

    #[serde(alias = "winget")]
    Winget,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::BsdPkg => Box::new(BsdPkg {}),
            PackageProviders::Homebrew => Box::new(Homebrew {}),
            PackageProviders::Yay => Box::new(Yay {}),
            PackageProviders::Winget => Box::new(Winget {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            // Arch Variants
            os_info::Type::Arch=> PackageProviders::Yay,
            os_info::Type::Manjaro=> PackageProviders::Yay,
            // BSD operating systems
            os_info::Type::DragonFly=> PackageProviders::BsdPkg,
            os_info::Type::FreeBSD=> PackageProviders::BsdPkg,
            // Debian / Ubuntu Variants
            os_info::Type::Debian => PackageProviders::Aptitude,
            os_info::Type::Mint => PackageProviders::Aptitude,
            os_info::Type::Pop => PackageProviders::Aptitude,
            os_info::Type::Ubuntu => PackageProviders::Aptitude,
            // For some reason, the Rust image is showing as this and
            // its Debian based?
            os_info::Type::OracleLinux => PackageProviders::Aptitude,
            // Other
            os_info::Type::Macos => PackageProviders::Homebrew,
            os_info::Type::Windows => PackageProviders::Winget,

            _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}

pub trait PackageProvider {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn bootstrap(&self) -> Vec<Step>;
    fn has_repository(&self, package: &PackageVariant) -> bool;
    fn add_repository(&self, package: &PackageVariant) -> Vec<Step>;
    fn query(&self, package: &PackageVariant) -> Vec<String>;
    fn install(&self, package: &PackageVariant) -> Vec<Step>;
}
