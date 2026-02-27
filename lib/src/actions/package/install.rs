use super::providers::PackageProviders;
use super::Package;
use super::PackageVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use std::ops::Deref;
use tracing::debug;
use tracing::span;

pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn summarize(&self) -> String {
        "Installing packages".to_string()
    }

    fn plan(&self, _manifest: &Manifest, context: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: PackageVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.install",
            provider = provider.name()
        )
        .entered();

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() {
            if provider.bootstrap(context).is_empty() {
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

            atoms.append(&mut provider.bootstrap(context));
        }

        atoms.append(&mut provider.install(&variant, context)?);

        span.exit();

        Ok(atoms)
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

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

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
