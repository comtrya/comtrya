//! `package.repository`: adds a package repository, and its signing key, to a package manager

use super::providers::PackageProviders;
use crate::actions::{Action, Error, PlanCx, Result};
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tracing::span;

/// Adds a package repository, unless the provider finds it already
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageRepository {
    /// The repository. For apt `<uri> <suite> [<component>...]` or a `ppa:`, for Homebrew a
    /// tap, for dnf the URL of a `.repo` file
    #[serde(alias = "url")]
    #[schemars(extend("x-aliases" = ["url"]))]
    pub name: String,

    /// The key the repository is signed with
    pub key: Option<RepositoryKey>,

    /// The package manager to use. Without it, `package_provider` from `Comtrya.yaml` or the
    /// default of the OS is used
    #[serde(default)]
    pub provider: Option<PackageProviders>,
}

/// The signing key of a repository
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositoryKey {
    /// Where to download the key from
    pub url: String,
    /// The file name of the key for apt, without `.asc`. Defaults to a hash of `url`
    pub name: Option<String>,
    /// Ignored with a warning
    pub key: Option<String>,
    /// Ignored with a warning. The key isn't checked against it
    pub fingerprint: Option<String>,
}

impl PackageRepository {
    /// The fields of the key that are set but not used
    pub(super) fn ignored_fields(&self) -> Vec<crate::actions::Ignored> {
        let Some(key) = &self.key else {
            return vec![];
        };

        let instead = "the key is downloaded from `key.url`";
        [
            ("key.key", key.key.is_some()),
            ("key.fingerprint", key.fingerprint.is_some()),
        ]
        .into_iter()
        .filter(|(_, set)| *set)
        .map(|(field, _)| (field, instead))
        .collect()
    }
}

impl Action for PackageRepository {
    fn summarize(&self) -> String {
        format!("Adding repository {}", self.name)
    }

    fn plan(&self, _manifest: &Manifest, cx: &PlanCx) -> Result<Vec<Step>> {
        let box_provider =
            PackageProviders::resolve(self.provider.as_ref(), cx.facts)?.get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.repository",
            provider = provider.name()
        )
        .entered();

        crate::actions::warn_ignored(&self.ignored_fields());

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() {
            if provider.bootstrap(cx.facts).is_empty() {
                return Err(Error::ProviderUnavailable {
                    provider: provider.name().to_string(),
                });
            }

            atoms.append(&mut provider.bootstrap(cx.facts));
        }

        if !provider.has_repository(self) {
            atoms.append(&mut provider.add_repository(self, cx.facts)?);
        }

        span.exit();

        Ok(atoms)
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::actions::Actions;

//     #[test]
//     fn it_can_be_deserialized() {
//         let yaml = r#"
// - action: package.install
//   name: curl

// - action: package.install
//   list:
//     - bash
// "#;

//         let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

//         match actions.pop() {
//             Some(Actions::PackageInstall(action)) => {
//                 assert_eq!(vec!["bash"], action.action.list);
//             }
//             _ => {
//                 panic!("PackageInstall didn't deserialize to the correct type");
//             }
//         };

//         match actions.pop() {
//             Some(Actions::PackageInstall(action)) => {
//                 assert_eq!("curl", action.action.name.unwrap());
//             }
//             _ => {
//                 panic!("PackageInstall didn't deserialize to the correct type");
//             }
//         };
//     }
// }
