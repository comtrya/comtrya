use super::Package;
use super::PackageVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;
use tracing::span;
use anyhow::anyhow;

pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
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
            if provider.bootstrap().is_empty() {
                return Err(anyhow!(
                    "Package Provider, {}, isn't available. Skipping action",
                    provider.name()
                ));                
            }

            atoms.append(&mut provider.bootstrap());
        }

        atoms.append(&mut provider.install(&variant));

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

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

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
