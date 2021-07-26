use super::Package;
use super::PackageVariant;
use crate::actions::Action;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;
use tera::Context;
use tracing::{error, span};

pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn plan(&self, _manifest: &Manifest, _context: &Context) -> Vec<Step> {
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
                error!(
                    "Package Provider, {}, isn't available. Skipping action",
                    provider.name()
                );
                return vec![];
            }

            atoms.append(&mut provider.bootstrap());
        }

        if let Some(ref _repo) = variant.repository {
            if !provider.has_repository(&variant) {
                atoms.append(&mut provider.add_repository(&variant));
            }
        }

        atoms.append(&mut provider.install(&variant));

        span.exit();

        atoms
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
    - curl
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!(vec!["curl"], action.action.names);
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };

        match actions.pop() {
            Some(Actions::PackageInstall(action)) => {
                assert_eq!(vec!["curl"], action.action.names);
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };
    }
}
