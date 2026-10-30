//! dnf on Fedora, Red Hat and CentOS

use super::PackageProvider;
use crate::actions::Result;

use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

/// dnf, as `dnf` or `yum`. Always runs `dnf install`, which skips installed packages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dnf {}

impl PackageProvider for Dnf {
    fn name(&self) -> &str {
        "DNF"
    }

    fn available(&self) -> bool {
        match which("dnf") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "dnf not available");
                false
            }
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("yum"),
            arguments: vec![
                String::from("install"),
                String::from("--assumeyes"),
                String::from("dnf"),
            ],
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, repository: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        if let Some(key) = repository.key.clone() {
            steps.extend(vec![Step::new(Exec {
                command: String::from("rpm"),
                arguments: vec![String::from("--import"), key.url],
                ..Default::default()
            })
            .as_root()]);
        }

        steps.extend(vec![
            Step::new(Exec {
                command: String::from("dnf"),
                arguments: vec![
                    String::from("config-manager"),
                    String::from("--assumeyes"),
                    String::from("--add-repo"),
                    repository.name.clone(),
                ],
                ..Default::default()
            })
            .as_root(),
            Step::new(Exec {
                command: String::from("dnf"),
                arguments: vec![
                    String::from("update"),
                    String::from("--assumeyes"),
                    String::from("--refresh"),
                ],
                ..Default::default()
            })
            .as_root(),
        ]);

        Ok(steps)
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("dnf"),
            arguments: vec![
                String::from("install"),
                String::from("--assumeyes"),
                String::from("--quiet"),
            ]
            .into_iter()
            .chain(package.extra_args.clone())
            .chain(self.query(package)?)
            .collect(),
            ..Default::default()
        })
        .as_root()])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::{providers::PackageProviders, repository::RepositoryKey};
    use crate::contexts::Facts;

    use super::*;

    #[test]
    fn test_add_repository_without_key() {
        let dnf = Dnf {};
        let facts = Facts::default();
        let steps = dnf.add_repository(
            &PackageRepository {
                name: String::from("test"),
                provider: Some(PackageProviders::Dnf),
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_repository_with_key() {
        let dnf = Dnf {};
        let facts = Facts::default();
        let steps = dnf.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    ..Default::default()
                }),
                provider: Some(PackageProviders::Dnf),
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }
}
