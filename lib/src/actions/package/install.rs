use super::providers::PackageProviders;
use super::Package;
use super::PackageVariant;
use super::PACKAGE_LOCK;
use crate::actions::Action;
use crate::actions::Actions;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::utilities::password_manager::PasswordManager;
use anyhow::anyhow;
use std::ops::Deref;
use tracing::debug;
use tracing::info_span;

pub type PackageInstall = Package;

#[async_trait::async_trait]
impl Action for PackageInstall {
    fn summarize(&self) -> String {
        "Installing packages".to_string()
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: PackageVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();
        let span = info_span!("package.install", provider = provider.name()).entered();
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

            if variant.file {
                match variant.provider {
                    PackageProviders::BsdPkg => debug!("Will attempt to install from local file."),
                    PackageProviders::Aptitude => {
                        debug!("Will attempt to install from local file.")
                    }
                    _ => {
                        return Err(anyhow!(
                            "Package Provider, {}, isn't capabale of local file installs. Skipping action.",
                            provider.name()
                        ));
                    }
                }
            }

            atoms.append(&mut bootstrap);
        }

        atoms.append(&mut provider.install(&variant, context)?);

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
        // Limit concurrent package installs to run exclusively of each-other
        let _permit = PACKAGE_LOCK.acquire().await?;
        Action::execute(self, dry_run, action, manifest, contexts, password_manager).await
    }

    fn is_privileged(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: package.install
  name: curl

- action: package.install
  list:
    - bash
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!(vec!["bash"], action.action.list);
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!("curl", action.action.name.unwrap());
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };
    }
}
