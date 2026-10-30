//! Snaps on Linux. comtrya installs snapd with apt when it's missing

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
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

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("apt"),
            arguments: vec![
                String::from("install"),
                String::from("--yes"),
                String::from("snapd"),
            ],
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _package: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _package: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("snap"),
            arguments: vec![String::from("install"), String::from("--yes")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            ..Default::default()
        })
        .as_root()])
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
