use super::Package;
use super::PackageVariant;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifests::Manifest;
use std::ops::Deref;
use tera::Context;
use tracing::span;

pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn run(&self, _manifest: &Manifest, _context: &Context) -> Result<ActionResult, ActionError> {
        let variant: PackageVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.install",
            provider = provider.name()
        )
        .entered();

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() && provider.bootstrap().is_err() {
            return Err(ActionError {
                message: String::from("Provider unavailable"),
            });
        }

        if let Some(ref _repo) = variant.repository {
            if !provider.has_repository(&variant) {
                if let Err(e) = provider.add_repository(&variant) {
                    return Err(e);
                }
            }
        }

        let result = match provider.install(&variant) {
            Ok(_) => Ok(ActionResult {
                message: String::from("Packages installed successfully"),
            }),
            Err(e) => Err(e),
        };

        span.exit();

        result
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
            Some(Actions::PackageInstall(package_install)) => {
                assert_eq!(vec!["curl"], package_install.list);
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };

        match actions.pop() {
            Some(Actions::PackageInstall(package_install)) => {
                assert_eq!("curl", package_install.name.unwrap());
            }
            _ => {
                panic!("PackageInstall didn't deserialize to the correct type");
            }
        };
    }
}
