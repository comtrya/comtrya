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
mod pacman;
use self::pacman::Pacman;
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
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{de::IntoDeserializer, Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    #[serde(rename = "pacman")]
    Pacman,

    #[serde(rename = "yay")]
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
            PackageProviders::Pacman => Box::new(Pacman {}),
            PackageProviders::Yay => Box::new(Yay {}),
            PackageProviders::Paru => Box::new(Paru {}),
            PackageProviders::Winget => Box::new(Winget {}),
            PackageProviders::Xbps => Box::new(Xbps {}),
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

#[cfg(unix)]
fn running_as_root() -> bool {
    uzers::get_current_uid() == 0
}

#[cfg(not(unix))]
fn running_as_root() -> bool {
    false
}

/// AUR helpers (`makepkg`) refuse to run as root, with an error that doesn't say what to do
fn refuse_root(provider: &str, contexts: &Contexts) -> anyhow::Result<()> {
    let root = contexts
        .get("user")
        .and_then(|user| user.get("id"))
        .is_some_and(|id| id.to_string() == "0");

    if root {
        return Err(anyhow!(
            "{provider} can't run as root. Run comtrya as a normal user, it asks for sudo where needed, or use `provider: pacman` for packages from the official repositories"
        ));
    }

    Ok(())
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

    fn user_contexts(id: &str) -> Contexts {
        let mut contexts = Contexts::default();
        contexts.insert(
            String::from("user"),
            [(String::from("id"), id.into())].into_iter().collect(),
        );
        contexts
    }

    #[test]
    fn aur_helpers_refuse_to_run_as_root() {
        let package = PackageVariant {
            name: Some(String::from("yay-only-package")),
            list: vec![],
            extra_args: vec![],
            provider: None,
            file: false,
        };

        for provider in [PackageProviders::Yay, PackageProviders::Paru] {
            let provider = provider.get_provider();

            let err = match provider.install(&package, &user_contexts("0")) {
                Ok(_) => panic!("{} installed as root", provider.name()),
                Err(err) => err.to_string(),
            };
            assert!(err.contains("can't run as root"), "{err}");
            assert!(err.contains("provider: pacman"), "{err}");
        }
    }

    #[test]
    fn root_is_detected_by_uid() {
        assert!(refuse_root("yay", &user_contexts("0")).is_err());
        assert!(refuse_root("yay", &user_contexts("1000")).is_ok());
        assert!(refuse_root("yay", &Contexts::default()).is_ok());
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
