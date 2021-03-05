use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Command};
use std::{io::Result, process::ExitStatus};

pub mod providers;
use providers::PackageProviders;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Package {
    name: Option<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    ensure: PackageStatus,

    #[serde(default)]
    list: Vec<String>,
}

pub trait PackageCommand {
    fn run_command(&self) -> (Result<ExitStatus>, Vec<u8>);
}

impl Package {
    pub fn name(&self) -> String {
        if self.name.is_some() {
            return self.name.clone().unwrap();
        }

        return self.list.join(" ");
    }

    pub fn get_package_list(&self) -> Vec<String> {
        if self.list.len() == 0 {
            return vec![self.name.clone().unwrap()];
        }

        return self.list.clone();
    }
}

// impl PackageCommand for Package {
//     fn run_command(&self) -> (Result<ExitStatus>, Vec<u8>) {
//         let mut command = match self.provider {
//             PackageProviders::Homebrew => Command::new("brew"),
//             PackageProviders::Apt => Command::new("apt"),
//         };

//         let command = match self.provider {
//             PackageProviders::Homebrew => command.arg("install").args(self.get_package_list()),
//             PackageProviders::Apt => command
//                 .args(&["install", "-y"])
//                 .args(self.get_package_list()),
//         };

//         match command.status() {
//             Ok(o) => (Ok(o), command.output().unwrap().stdout),
//             Err(e) => (Err(e), command.output().unwrap().stderr),
//         }
//     }
// }

//////////////////////////////////////////////

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageStatus {
    Installed,
    Latest,
    Uninstalled,
}

impl Default for PackageStatus {
    fn default() -> Self {
        PackageStatus::Installed
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackageConfig {
    name: Option<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    ensure: PackageStatus,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, Package>,
}

#[derive(Clone, Debug)]
pub struct ProviderPackage {
    name: Option<String>,
    provider: PackageProviders,
    repository: Option<String>,
}

impl From<PackageConfig> for ProviderPackage {
    fn from(package: PackageConfig) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = package.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return ProviderPackage {
                name: package.name,
                provider: package.provider,
                repository: package.repository,
            };
        };

        let variant = variant.unwrap();

        let mut provider_package = ProviderPackage {
            name: package.name,
            provider: package.provider,
            repository: package.repository,
        };

        if variant.name.is_some() {
            provider_package.name = variant.name.clone();
        };

        if variant.repository.is_some() {
            provider_package.repository = variant.repository.clone();
        };

        // I've been torn about this, but here's my logic.
        // Variants, when being used, shouldn't use the provider
        // of the main definition; as we're not the core OS.
        // Even if the omission of a provider for a variant gets us
        // the default, that's most likely still expected behaviour.
        // Right?
        provider_package.provider = variant.provider.clone();

        provider_package
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_defaults_to_installed() {
        let json = r#"name: my_package"#;
        let package: Package = serde_yaml::from_str(json).unwrap();
        assert_eq!(package.ensure, PackageStatus::Installed);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn provider_defaults_to_homebrew_on_macos() {
        let json = r#"name: my_package"#;
        let package: Package = serde_yaml::from_str(json).unwrap();
        assert_eq!(package.provider, PackageProviders::Homebrew);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn provider_defaults_for_linux() {
        let json = r#"name: my_package"#;
        let package: Package = serde_yaml::from_str(json).unwrap();
        assert_eq!(package.provider, PackageProviders::Scoop);
    }

    // I can't really mock the os_info::get() call
    // So not sure how to test I get the correct default package
    // provider for each OS?
    //
    // #[cfg(target_os = "linux")]
    // #[test]
    // fn provider_defaults_for_linux() {
    //     let json = r#"name: my_package"#;
    //     let package: Package = serde_yaml::from_str(json).unwrap();
    //     assert_eq!(package.provider, PackageProviders::Homebrew);
    // }
}
