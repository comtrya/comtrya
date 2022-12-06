use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

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

    fn bootstrap(&self) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _repository: &PackageRepository) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("zypper"),
                arguments: vec![String::from("install"), String::from("-y")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::providers::PackageProviders;

    use super::*;

    #[test]
    fn test_install() {
        let zypper = Zypper {};
        let steps = zypper.install(&PackageVariant {
            name: Some(String::from("")),
            list: vec![],
            extra_args: vec![],
            provider: PackageProviders::Zypper,
        });

        assert_eq!(steps.unwrap().len(), 1);
    }
}
