//! Snaps on Linux. comtrya installs snapd with apt when it's missing

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec, utilities};
use serde::{Deserialize, Serialize};
use std::default::Default;
use tracing::warn;
use which::which;

/// snap, as `snapcraft` or `snap`. Always runs `snap install`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapcraft {}

impl PackageProvider for Snapcraft {
    fn name(&self) -> &str {
        "Snapcraft"
    }

    fn available(&self) -> bool {
        match which("snap") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "snap is not available");
                false
            }
        }
    }

    fn bootstrap(&self, facts: &Facts) -> Vec<Step> {
        let privilege_provider = utilities::get_privilege_provider(facts);

        vec![Step::new(Exec {
            command: String::from("apt"),
            arguments: vec![
                String::from("install"),
                String::from("--yes"),
                String::from("snapd"),
            ],
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        })]
    }

    fn has_repository(&self, _package: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        _package: &PackageRepository,
        _facts: &Facts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, facts: &Facts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider = utilities::get_privilege_provider(facts);
        Ok(vec![Step::new(Exec {
            command: String::from("snap"),
            arguments: vec![String::from("install"), String::from("--yes")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        })])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::providers::PackageProviders;
    use crate::contexts::Facts;

    use super::*;

    #[test]
    fn test_install() {
        let snapcraft = Snapcraft {};
        let facts = Facts::default();
        let steps = snapcraft.install(
            &PackageVariant {
                name: Some(String::from("")),
                list: vec![],
                extra_args: vec![],
                provider: Some(PackageProviders::Snapcraft),
                file: false,
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }
}
