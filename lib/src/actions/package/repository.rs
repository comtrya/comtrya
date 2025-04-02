use super::providers::PackageProviders;
use super::PACKAGE_LOCK;
use crate::actions::{Action, Actions};
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities::password_manager::PasswordManager;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use tracing::span;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageRepository {
    #[serde(alias = "url")]
    pub name: String,

    pub key: Option<RepositoryKey>,

    #[serde(default)]
    pub provider: PackageProviders,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositoryKey {
    pub url: String,
    pub name: Option<String>,
    pub key: Option<String>,
    pub fingerprint: Option<String>,
}

#[async_trait::async_trait]
impl Action for PackageRepository {
    fn summarize(&self) -> String {
        format!("Adding repository {}", self.name)
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let box_provider = self.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.repository",
            provider = provider.name()
        )
        .entered();

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() {
            let mut bootstrap = provider.bootstrap(context);

            if bootstrap.is_empty() {
                return Err(anyhow!(
                    "Package Provider, {}, isn't available. Skipping action",
                    provider.name()
                ));
            }

            atoms.append(&mut bootstrap);
        }

        if !provider.has_repository(self) {
            atoms.append(&mut provider.add_repository(self, context)?);
        }

        span.exit();

        Ok(atoms)
    }

    async fn execute(
        &self,
        dry_run: bool,
        action: &Actions,
        manifest: &Manifest,
        contexts: &Contexts,
        password_manager: Option<PasswordManager>,
    ) -> anyhow::Result<()> {
        // Limit concurrent package repository execs to run exclusively of each-other
        let _permit = PACKAGE_LOCK.acquire().await?;
        Action::execute(self, dry_run, action, manifest, contexts, password_manager).await
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
