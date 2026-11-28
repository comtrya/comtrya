//! `package.install`

use super::providers::{PackageProvider, PackageProviders};
use super::Package;
use super::{PackageState, PackageVariant};
use crate::actions::{Action, Error, PlanCx, Result};
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;
use tracing::{debug, span};

/// The `package.install` action
pub type PackageInstall = Package;

impl Action for PackageInstall {
    fn summarize(&self) -> String {
        let verb = match self.state() {
            PackageState::Present => "Install",
            PackageState::Latest => "Install or upgrade",
            PackageState::Absent => "Remove",
        };
        let packages = PackageVariant::from(self).packages();
        if packages.is_empty() {
            return format!("{verb} packages");
        }
        format!("{verb} {}", packages.join(", "))
    }

    fn plan(&self, _manifest: &Manifest, cx: &PlanCx) -> Result<Vec<Step>> {
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

        if variant.file && self.state() != PackageState::Present {
            return Err(Error::Invalid {
                field: "file",
                problem: format!(
                    "only works with `state: present`, not `{}`",
                    self.state().as_str()
                ),
            });
        }

        if variant.file && !installs_local_files(&provider_type) {
            return Err(Error::NoLocalInstall {
                provider: provider.name().to_string(),
            });
        }

        let steps = match self.state() {
            PackageState::Absent => remove(provider, &variant, cx),
            state => install(provider, &variant, cx, state),
        };

        span.exit();

        steps
    }
}

/// Installs the packages, or upgrades them for `latest`. A provider that isn't installed is
/// bootstrapped first, then there's nothing to upgrade
fn install(
    provider: &dyn PackageProvider,
    variant: &PackageVariant,
    cx: &PlanCx,
    state: PackageState,
) -> Result<Vec<Step>> {
    if provider.available() {
        return match state {
            PackageState::Latest => provider.upgrade(variant, cx.facts),
            _ => provider.install(variant, cx.facts),
        };
    }

    let mut steps = provider.bootstrap(cx.facts);
    if steps.is_empty() {
        return Err(Error::ProviderUnavailable {
            provider: provider.name().to_string(),
        });
    }

    steps.append(&mut provider.install(variant, cx.facts)?);
    Ok(steps)
}

/// Removes the packages. Without the provider, none of its packages are installed
fn remove(
    provider: &dyn PackageProvider,
    variant: &PackageVariant,
    cx: &PlanCx,
) -> Result<Vec<Step>> {
    if !provider.available() {
        debug!(
            "{} isn't installed, so neither are the packages",
            provider.name()
        );
        return Ok(vec![]);
    }

    provider.remove(variant, cx.facts)
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
    fn the_summary_names_the_packages() {
        let one = PackageInstall {
            name: Some(String::from("git")),
            ..Default::default()
        };
        let several = PackageInstall {
            list: vec![String::from("git"), String::from("vim")],
            ..Default::default()
        };

        assert_eq!("Install git", one.summarize());
        assert_eq!("Install git, vim", several.summarize());
        assert_eq!("Install packages", PackageInstall::default().summarize());
    }

    #[test]
    fn the_summary_says_what_the_state_does() {
        let package = |state| PackageInstall {
            name: Some(String::from("git")),
            state: Some(state),
            ..Default::default()
        };

        assert_eq!("Install git", package(PackageState::Present).summarize());
        assert_eq!(
            "Install or upgrade git",
            package(PackageState::Latest).summarize()
        );
        assert_eq!("Remove git", package(PackageState::Absent).summarize());
    }

    #[test]
    fn local_files_only_work_with_state_present() {
        let package = PackageInstall {
            name: Some(String::from("/tmp/nano.deb")),
            provider: Some(PackageProviders::Aptitude),
            file: true,
            state: Some(PackageState::Absent),
            ..Default::default()
        };

        let Err(error) = package.plan(&Manifest::default(), &PlanCx::empty()) else {
            panic!("file: true can't be removed");
        };

        assert!(error.to_string().contains("state: present"), "{error}");
    }

    #[test]
    fn state_is_read_from_manifests_and_defaults_to_present() {
        let yaml = r"
- action: package.install
  name: git
  state: latest

- action: package.install
  name: vim
";
        let actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();
        let states: Vec<PackageState> = actions
            .iter()
            .map(|action| match action {
                Actions::PackageInstall(action) => action.action.state(),
                _ => panic!("not package.install"),
            })
            .collect();

        assert_eq!(vec![PackageState::Latest, PackageState::Present], states);
        assert!(serde_yaml_ng::from_str::<Vec<Actions>>(
            "- action: package.install\n  name: git\n  state: newest\n"
        )
        .is_err());
    }

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
        let yaml = r"
- action: package.install
  name: curl

- action: package.install
  list:
    - bash
";

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
