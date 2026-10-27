//! The package managers behind `package.install` and `package.repository`, and how the one
//! to use is chosen

mod aptitude;
use self::aptitude::Aptitude;
mod aur;
use self::aur::AurHelper;
use crate::steps::Step;
mod bsdpkg;
use self::bsdpkg::BsdPkg;
mod dnf;
use self::dnf::Dnf;
mod homebrew;
use self::homebrew::Homebrew;
mod macports;
use self::macports::Macports;
mod pacman;
use self::pacman::Pacman;
mod pkgin;
use self::pkgin::Pkgin;
mod snapcraft;
use self::snapcraft::Snapcraft;
mod winget;
use self::winget::Winget;
mod xbps;
use self::xbps::Xbps;
mod zypper;
use self::zypper::Zypper;
use super::{repository::PackageRepository, PackageVariant};
use crate::contexts::Contexts;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use tracing::{debug, trace};

/// A package manager, by the name used for `provider` in manifests and `Comtrya.yaml`
#[derive(JsonSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PackageProviders {
    /// apt on Debian and Ubuntu
    #[serde(rename = "aptitude", alias = "apt", alias = "apt-get")]
    Aptitude,

    /// pkg on FreeBSD and DragonFly
    #[serde(rename = "bsdpkg")]
    BsdPkg,

    /// dnf on Fedora and Red Hat
    #[serde(rename = "dnf", alias = "yum")]
    Dnf,

    /// Homebrew on macOS
    #[serde(rename = "homebrew", alias = "brew")]
    Homebrew,

    /// MacPorts on macOS
    #[serde(rename = "macports", alias = "port")]
    Macports,

    /// pkgin on NetBSD
    #[serde(rename = "pkgin")]
    Pkgin,

    /// Snaps on Linux
    #[serde(rename = "snapcraft", alias = "snap")]
    Snapcraft,

    /// pacman on Arch, official repositories only
    #[serde(rename = "pacman")]
    Pacman,

    /// yay on Arch, with the AUR
    #[serde(rename = "yay")]
    Yay,

    /// paru on Arch, with the AUR
    #[serde(rename = "paru")]
    Paru,

    /// winget on Windows
    #[serde(rename = "winget")]
    Winget,

    /// xbps on Void Linux
    #[serde(rename = "xbps")]
    Xbps,

    /// zypper on openSUSE
    #[serde(rename = "zypper")]
    Zypper,
}

impl PackageProviders {
    /// The implementation of this package manager
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Aptitude => Box::new(Aptitude {}),
            PackageProviders::BsdPkg => Box::new(BsdPkg {}),
            PackageProviders::Dnf => Box::new(Dnf {}),
            PackageProviders::Homebrew => Box::new(Homebrew::default()),
            PackageProviders::Macports => Box::new(Macports {}),
            PackageProviders::Pkgin => Box::new(Pkgin {}),
            PackageProviders::Pacman => Box::new(Pacman::default()),
            PackageProviders::Yay => Box::new(AurHelper::yay()),
            PackageProviders::Paru => Box::new(AurHelper::paru()),
            PackageProviders::Winget => Box::new(Winget::default()),
            PackageProviders::Xbps => Box::new(Xbps::default()),
            PackageProviders::Zypper => Box::new(Zypper {}),
            PackageProviders::Snapcraft => Box::new(Snapcraft {}),
        }
    }

    /// The name used in manifests and `Comtrya.yaml`
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageProviders::Aptitude => "aptitude",
            PackageProviders::BsdPkg => "bsdpkg",
            PackageProviders::Dnf => "dnf",
            PackageProviders::Homebrew => "homebrew",
            PackageProviders::Macports => "macports",
            PackageProviders::Pkgin => "pkgin",
            PackageProviders::Snapcraft => "snapcraft",
            PackageProviders::Pacman => "pacman",
            PackageProviders::Yay => "yay",
            PackageProviders::Paru => "paru",
            PackageProviders::Winget => "winget",
            PackageProviders::Xbps => "xbps",
            PackageProviders::Zypper => "zypper",
        }
    }

    /// The provider an action uses: its own `provider`, else `packages.provider` from the
    /// contexts (`package_provider` in `Comtrya.yaml`), else the default for this OS
    pub fn resolve(
        provider: Option<&PackageProviders>,
        contexts: &Contexts,
    ) -> anyhow::Result<Self> {
        if let Some(provider) = provider {
            return Ok(provider.clone());
        }

        if let Some(name) = contexts
            .get("packages")
            .and_then(|packages| packages.get("provider"))
        {
            let name = name.to_string();
            return PackageProviders::deserialize(name.as_str().into_deserializer()).map_err(
                |err: serde::de::value::Error| anyhow!("Invalid package provider: {err}"),
            );
        }

        PackageProviders::for_os().ok_or_else(|| {
            anyhow!(
                "There is no default package provider for {}. Set `provider` on the action, or `package_provider` in Comtrya.yaml",
                os_info::get().os_type()
            )
        })
    }

    /// The usual package provider of this OS, if comtrya knows one
    pub fn for_os() -> Option<Self> {
        let info = os_info::get();

        tracing::trace!("Detected OS for the default package provider: {info:?}");

        let provider = match info.os_type() {
            // Arch Variants
            os_info::Type::Arch
            | os_info::Type::Artix
            | os_info::Type::CachyOS
            | os_info::Type::EndeavourOS
            | os_info::Type::Manjaro => {
                Self::for_arch(|command| which::which(command).is_ok(), running_as_root())
            }
            // BSD operating systems
            os_info::Type::DragonFly => PackageProviders::BsdPkg,
            os_info::Type::FreeBSD => PackageProviders::BsdPkg,
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

            _ => return None,
        };

        Some(provider)
    }

    /// pacman as root, because AUR helpers refuse to run as root. Otherwise the AUR helper that is
    /// installed, else yay, which is bootstrapped
    fn for_arch(installed: impl Fn(&str) -> bool, root: bool) -> Self {
        if root {
            PackageProviders::Pacman
        } else if installed("paru") {
            PackageProviders::Paru
        } else {
            PackageProviders::Yay
        }
    }
}

/// Whether comtrya runs as root, by its uid
#[cfg(unix)]
fn running_as_root() -> bool {
    uzers::get_current_uid() == 0
}

/// Windows has no root
#[cfg(not(unix))]
fn running_as_root() -> bool {
    false
}

/// The requested packages that `installed` doesn't find, in the order they were requested
fn missing(package: &PackageVariant, installed: impl Fn(&str) -> bool) -> Vec<String> {
    package
        .packages()
        .into_iter()
        .filter(|p| {
            if installed(p) {
                trace!("{}: already installed", p);
                false
            } else {
                debug!("{}: doesn't appear to be installed", p);
                true
            }
        })
        .collect()
}

/// A package manager: what `package.install` and `package.repository` plan their steps with
pub trait PackageProvider {
    /// The name shown in the output
    fn name(&self) -> &str;
    /// Whether the package manager is installed
    fn available(&self) -> bool;
    /// Steps that install the package manager itself. Empty if comtrya can't install it, then
    /// the action fails when it isn't `available`
    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step>;
    /// Whether the repository is set up already, so nothing needs to be done
    fn has_repository(&self, package: &PackageRepository) -> bool;
    /// Steps that add the repository and its key
    fn add_repository(
        &self,
        package: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>>;
    /// The requested packages that aren't installed. Providers that can't tell return all of
    /// them
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>>;
    /// Steps that install the packages that are missing
    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, contexts::build_contexts};

    fn contexts(package_provider: Option<PackageProviders>) -> Contexts {
        build_contexts(&Config {
            package_provider,
            ..Default::default()
        })
    }

    #[test]
    fn the_action_provider_wins() {
        let contexts = contexts(Some(PackageProviders::Pkgin));
        let provider = PackageProviders::resolve(Some(&PackageProviders::Xbps), &contexts).unwrap();

        assert_eq!(provider, PackageProviders::Xbps);
    }

    #[test]
    fn the_configured_provider_replaces_the_os_default() {
        let contexts = contexts(Some(PackageProviders::Pkgin));

        assert_eq!(
            PackageProviders::resolve(None, &contexts).unwrap(),
            PackageProviders::Pkgin
        );
    }

    #[test]
    fn without_configuration_the_os_default_is_used() {
        let contexts = contexts(None);

        match PackageProviders::for_os() {
            Some(default) => {
                assert_eq!(PackageProviders::resolve(None, &contexts).unwrap(), default)
            }
            None => assert!(PackageProviders::resolve(None, &contexts).is_err()),
        }
    }

    #[test]
    fn names_deserialize_to_the_same_provider() {
        let providers = [
            PackageProviders::Aptitude,
            PackageProviders::BsdPkg,
            PackageProviders::Dnf,
            PackageProviders::Homebrew,
            PackageProviders::Macports,
            PackageProviders::Pkgin,
            PackageProviders::Snapcraft,
            PackageProviders::Pacman,
            PackageProviders::Yay,
            PackageProviders::Paru,
            PackageProviders::Winget,
            PackageProviders::Xbps,
            PackageProviders::Zypper,
        ];

        for provider in providers {
            let parsed: PackageProviders = serde_yaml_ng::from_str(provider.as_str()).unwrap();
            assert_eq!(parsed, provider);
        }
    }

    #[test]
    fn aliases_are_accepted_in_the_config() {
        let config: Config = serde_yaml_ng::from_str("package_provider: brew").unwrap();

        assert_eq!(config.package_provider, Some(PackageProviders::Homebrew));
        assert!(serde_yaml_ng::from_str::<Config>("package_provider: nope").is_err());
    }

    /// `pacman` used to be an alias of yay, now it's its own provider for the official repositories
    #[test]
    fn pacman_is_not_yay() {
        let parsed: PackageProviders = serde_yaml_ng::from_str("pacman").unwrap();

        assert_eq!(parsed, PackageProviders::Pacman);
    }

    #[test]
    fn arch_uses_pacman_as_root() {
        assert_eq!(
            PackageProviders::for_arch(|_| true, true),
            PackageProviders::Pacman
        );
        assert_eq!(
            PackageProviders::for_arch(|_| false, true),
            PackageProviders::Pacman
        );
    }

    #[test]
    fn arch_prefers_an_installed_paru_over_yay() {
        assert_eq!(
            PackageProviders::for_arch(|c| c == "paru", false),
            PackageProviders::Paru
        );
        assert_eq!(
            PackageProviders::for_arch(|_| true, false),
            PackageProviders::Paru
        );
        assert_eq!(
            PackageProviders::for_arch(|c| c == "yay", false),
            PackageProviders::Yay
        );
        assert_eq!(
            PackageProviders::for_arch(|_| false, false),
            PackageProviders::Yay
        );
    }
}
