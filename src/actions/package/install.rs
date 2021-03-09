use super::Package;
use super::PackageVariant;
use crate::actions::command::CommandAction;
use crate::actions::{Action, ActionError, ActionResult};
use crate::manifest::Manifest;
use std::ops::Deref;
use tera::Context;
use tracing::error;

pub type PackageInstall = Package;

impl CommandAction for PackageInstall {}

impl Action for PackageInstall {
    fn run(&self, _manifest: &Manifest, _context: &Context) -> Result<ActionResult, ActionError> {
        let variant: PackageVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() && provider.bootstrap().is_err() {
            return Err(ActionError {
                message: String::from("Provider unavailable"),
            });
        }

        if let Some(ref repo) = variant.repository {
            if !provider.has_repository(&repo) {
                if let Err(e) = provider.add_repository(&repo) {
                    return Err(e);
                }
            }
        }

        match provider.install(variant.packages()) {
            Ok(_) => (),
            Err(e) => {
                error!(
                    message = "Failed to install package",
                    packages = variant.packages().join(",").as_str()
                );
                return Err(e);
            }
        }

        Ok(ActionResult {
            message: String::from("Done"),
        })
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
