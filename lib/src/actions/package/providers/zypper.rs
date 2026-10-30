//! zypper on openSUSE and SUSE

use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::actions::Result;
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

/// zypper. Always runs `zypper install`, which skips installed packages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zypper {}

impl PackageProvider for Zypper {
    fn name(&self) -> &str {
        "Zypper"
    }

    fn available(&self) -> bool {
        match which("zypper") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "zypper not available");
                false
            }
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _repository: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("zypper"),
            arguments: vec![String::from("install"), String::from("-y")]
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
        let zypper = Zypper {};
        let facts = Facts::default();
        let steps = zypper.install(
            &PackageVariant {
                name: Some(String::from("")),
                list: vec![],
                extra_args: vec![],
                provider: Some(PackageProviders::Zypper),
                file: false,
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }
}
