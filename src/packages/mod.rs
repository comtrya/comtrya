use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod providers;
use providers::PackageProviders;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Package {
    #[serde(default)]
    pub list: Vec<String>,

    #[serde(default)]
    pub ensure: PackageStatus,

    #[serde(default)]
    pub provider: PackageProviders,

    #[serde(default)]
    pub repository: Option<String>,
}

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
    list: Vec<String>,

    #[serde(default)]
    ensure: PackageStatus,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, Variant>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,
}

impl PackageConfig {
    pub fn get_list(self) -> Vec<String> {
        if self.name.is_some() {
            return vec![self.name.unwrap()];
        }

        if self.list.is_empty() {
            return vec![];
        }

        return self.list;
    }
}

impl Variant {
    pub fn get_list(self) -> Vec<String> {
        if self.name.is_some() {
            return vec![self.name.unwrap()];
        }

        if self.list.is_empty() {
            return vec![];
        }

        return self.list;
    }
}

impl From<PackageConfig> for Package {
    fn from(package_config: PackageConfig) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = package_config.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return Package {
                list: package_config.clone().get_list(),
                ensure: package_config.clone().ensure,
                provider: package_config.clone().provider,
                repository: package_config.clone().repository,
            };
        };

        let variant = variant.unwrap();

        let mut package = Package {
            list: package_config.clone().get_list(),
            ensure: package_config.ensure,
            provider: package_config.provider,
            repository: package_config.repository,
        };

        if false == variant.clone().get_list().is_empty() {
            package.list = variant.clone().get_list();
        }

        if variant.repository.is_some() {
            package.repository = variant.repository.clone();
        };

        // I've been torn about this, but here's my logic.
        // Variants, when being used, shouldn't use the provider
        // of the main definition; as we're not the core OS.
        // Even if the omission of a provider for a variant gets us
        // the default, that's most likely still expected behaviour.
        // Right?
        package.provider = variant.provider.clone();

        package
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

pub fn get_provider(provider: PackageProviders) -> Option<Box<dyn providers::PackageProvider>> {
    match provider {
        PackageProviders::Apt => Some(Box::new(providers::apt::Aptitude {})),
        PackageProviders::Homebrew => Some(Box::new(providers::homebrew::Homebrew {})),
        _ => None,
    }
}
