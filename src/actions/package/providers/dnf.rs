use super::PackageProvider;

use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    fn bootstrap(&self) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("yum"),
                arguments: vec![
                    String::from("install"),
                    String::from("--assumeyes"),
                    String::from("dnf"),
                ],
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, package: &PackageVariant) -> Vec<Step> {
        if package.repository.is_none() {
            return vec![];
        }

        let mut steps: Vec<Step> = vec![];

        if package.key.is_some() {
            steps.extend(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("rpm"),
                    arguments: vec![String::from("--import"), package.key.clone().unwrap()],
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        steps.extend(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("dnf"),
                    arguments: vec![
                        String::from("config-manager"),
                        String::from("--assumeyes"),
                        String::from("--add-repo"),
                        package.repository.clone().unwrap(),
                    ],
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("dnf"),
                    arguments: vec![
                        String::from("update"),
                        String::from("--assumeyes"),
                        String::from("--refresh"),
                    ],
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]);

        steps
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("dnf"),
                arguments: vec![
                    String::from("install"),
                    String::from("--assumeyes"),
                    String::from("--quiet"),
                ]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(self.query(package))
                .collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_repository_simple() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            ..Default::default()
        };

        let dnf = Dnf {};
        let steps = dnf.add_repository(&package);

        assert_eq!(steps.len(), 0);
    }

    #[test]
    fn test_add_repository_without_key() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            repository: Some(String::from("repository")),
            ..Default::default()
        };

        let dnf = Dnf {};
        let steps = dnf.add_repository(&package);

        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_repository_with_key() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            repository: Some(String::from("repository")),
            key: Some(String::from("key")),
            ..Default::default()
        };

        let dnf = Dnf {};
        let steps = dnf.add_repository(&package);

        assert_eq!(steps.len(), 3);
    }
}
