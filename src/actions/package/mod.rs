pub mod install;
pub mod providers;

use providers::PackageProviders;
use serde::de::{self, SeqAccess};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing::debug;

fn deserialize_name_or_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct SingleOrManyNames;

    impl<'de> de::Visitor<'de> for SingleOrManyNames {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("either a string or a list of strings")
        }

        // We got a single string in "name"...
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(vec![v.to_string()])
        }

        // or a list of names in "list"
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut res = Vec::new();
            while let Ok(Some(element)) = seq.next_element::<String>() {
                res.push(element);
            }
            Ok(res)
        }
    }

    deserializer.deserialize_any(SingleOrManyNames)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Package {
    #[serde(
        deserialize_with = "deserialize_name_or_list",
        alias = "name",
        alias = "list"
    )]
    names: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    extra_args: Vec<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, PackageVariant>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageVariant {
    #[serde(
        deserialize_with = "deserialize_name_or_list",
        alias = "name",
        alias = "list"
    )]
    names: Vec<String>,

    #[serde(default)]
    provider: PackageProviders,

    #[serde(default)]
    repository: Option<String>,

    #[serde(default)]
    extra_args: Vec<String>,
}

impl PackageVariant {
    fn packages(&self) -> Vec<String> {
        self.names.clone()
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
                names: package.names.clone(),
                provider: package.provider.clone(),
                repository: package.repository.clone(),
                extra_args: package.extra_args.clone(),
            };
        };

        let variant = variant.unwrap();

        debug!(message = "Built Variant", variant = ?variant);

        let mut package = PackageVariant {
            names: variant.names.clone(),
            provider: variant.provider.clone(),
            repository: variant.repository.clone(),
            extra_args: variant.extra_args.clone(),
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
