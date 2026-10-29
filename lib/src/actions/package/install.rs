//! `package.install`

use super::providers::PackageProviders;
use super::Package;
use super::PackageVariant;
use crate::actions::Action;
use crate::actions::PlanCx;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use std::ops::Deref;
use tracing::span;

/// The `package.install` action
pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn summarize(&self) -> String {
        "Installing packages".to_string()
    }

    fn plan(&self, _manifest: &Manifest, cx: &PlanCx) -> anyhow::Result<Vec<Step>> {
        let variant: PackageVariant = self.into();
        let provider_type = PackageProviders::resolve(variant.provider.as_ref(), cx.facts)?;
        let box_provider = provider_type.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "package.install",
            provider = provider.name()
        )
        .entered();

        crate::actions::warn_ignored(&self.ignored_fields());

        if variant.file && !installs_local_files(&provider_type) {
            return Err(anyhow!(
                "Package Provider, {}, isn't capable of local file installs. Skipping action.",
                provider.name()
            ));
        }

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        if !provider.available() {
            if provider.bootstrap(cx.facts).is_empty() {
                return Err(anyhow!(
                    "Package Provider, {}, isn't available. Skipping action",
                    provider.name()
                ));
            }

            atoms.append(&mut provider.bootstrap(cx.facts));
        }

        atoms.append(&mut provider.install(&variant, cx.facts)?);

        span.exit();

        Ok(atoms)
    }
}

/// Whether the provider installs a package from a file given as `name`, for `file: true`
fn installs_local_files(provider: &PackageProviders) -> bool {
    matches!(
        provider,
        PackageProviders::BsdPkg | PackageProviders::Aptitude
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Actions;

    #[test]
    fn local_files_fail_with_a_provider_that_cant_install_them() {
        let package = PackageInstall {
            name: Some(String::from("/tmp/nano-8.1.pkg")),
            provider: Some(PackageProviders::Homebrew),
            file: true,
            ..Default::default()
        };

        let Err(error) = package.plan(&Manifest::default(), &PlanCx::empty()) else {
            panic!("homebrew can't install local files");
        };

        assert!(error.to_string().contains("local file"), "{error}");
    }

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
