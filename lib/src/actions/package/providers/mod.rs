mod aptitude;
use self::aptitude::Aptitude;
use crate::steps::Step;
mod bsdpkg;
use self::bsdpkg::BsdPkg;
mod dnf;
use self::dnf::Dnf;
mod homebrew;
use self::homebrew::Homebrew;
mod macports;
use self::macports::Macports;
mod paru;
use self::paru::Paru;
mod pkgin;
use self::pkgin::Pkgin;
mod snapcraft;
use self::snapcraft::Snapcraft;
mod yay;
use self::yay::Yay;
mod winget;
use self::winget::Winget;
mod xbps;
use self::xbps::Xbps;
mod zypper;
use self::zypper::Zypper;
use super::{repository::PackageRepository, PackageVariant};
use crate::contexts::Contexts;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(rename = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,

    #[serde(rename = "bsdpkg")]
    BsdPkg,

    #[serde(rename = "dnf", alias = "yum")]
    Dnf,

    #[serde(rename = "homebrew", alias = "brew")]
    Homebrew,

    #[serde(rename = "macports", alias = "port")]
    Macports,

    #[serde(rename = "pkgin")]
    Pkgin,

    #[serde(rename = "snapcraft", alias = "snap")]
    Snapcraft,

    #[serde(rename = "yay", alias = "pacman")]
    Yay,

    #[serde(rename = "paru")]
    Paru,

    #[serde(rename = "winget")]
    Winget,

    #[serde(rename = "xbps")]
    Xbps,

    #[serde(rename = "zypper")]
    Zypper,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::BsdPkg => Box::new(BsdPkg {}),
            PackageProviders::Dnf => Box::new(Dnf {}),
            PackageProviders::Homebrew => Box::new(Homebrew {}),
            PackageProviders::Macports => Box::new(Macports {}),
            PackageProviders::Pkgin => Box::new(Pkgin {}),
            PackageProviders::Yay => Box::new(Yay {}),
            PackageProviders::Paru => Box::new(Paru {}),
            PackageProviders::Winget => Box::new(Winget {}),
            PackageProviders::Xbps => Box::new(Xbps {}),
            PackageProviders::Zypper => Box::new(Zypper {}),
            PackageProviders::Snapcraft => Box::new(Snapcraft {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        println!("Info: {info:?}");

        match info.os_type() {
            // Arch Variants
            os_info::Type::Arch=> PackageProviders::Yay,
            os_info::Type::Artix => PackageProviders::Yay,
            os_info::Type::CachyOS => PackageProviders::Yay,
            os_info::Type::EndeavourOS => PackageProviders::Yay,
            os_info::Type::Manjaro=> PackageProviders::Yay,
            // BSD operating systems
            os_info::Type::DragonFly=> PackageProviders::BsdPkg,
            os_info::Type::FreeBSD=> PackageProviders::BsdPkg,
            os_info::Type::NetBSD => PackageProviders::Pkgin,
            // Debian / Ubuntu Variants
            os_info::Type::Debian => PackageProviders::Aptitude,
            os_info::Type::Mint => PackageProviders::Aptitude,
            os_info::Type::Pop => PackageProviders::Aptitude,
            os_info::Type::Ubuntu => PackageProviders::Aptitude,
            // For some reason, the Rust image is showing as this and
            // its Debian based?
            os_info::Type::OracleLinux => PackageProviders::Aptitude,
            // OpenSUSE and SUSE
            os_info::Type::openSUSE => PackageProviders::Zypper,
            os_info::Type::SUSE => PackageProviders::Zypper,
            // Red-Hat Variants
            os_info::Type::Fedora => PackageProviders::Dnf,
            os_info::Type::Redhat => PackageProviders::Dnf,
            os_info::Type::RedHatEnterprise => PackageProviders::Dnf,
            os_info::Type::CentOS => PackageProviders::Dnf,
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
    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step>;
    fn has_repository(&self, package: &PackageRepository) -> bool;
    fn add_repository(
        &self,
        package: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>>;
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>>;
    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
}
