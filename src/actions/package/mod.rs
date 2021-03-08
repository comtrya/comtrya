use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod install;
pub mod providers;
use providers::PackageProviders;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Package {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, PackageVariant>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageVariant {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,
}

impl PackageVariant {
    fn packages(&self) -> Vec<String> {
        if self.name.is_some() {
            return vec![self.name.clone().unwrap()];
        }

        if self.list.is_empty() {
            return vec![];
        }

        return self.list.clone();
    }
}

impl From<&Package> for PackageVariant {
    fn from(package: &Package) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = package.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return PackageVariant {
                name: package.name.clone(),
                list: package.list.clone(),
                provider: package.provider.clone(),
                repository: package.repository.clone(),
            };
        };

        let variant = variant.unwrap();

        print!("Found a variant: {:?}", variant);

        let mut package = PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: package.provider.clone(),
            repository: package.repository.clone(),
        };

        if variant.name.is_some() {
            package.name = variant.name.clone();
        }

        if false == variant.list.is_empty() {
            package.list = variant.list.clone();
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
