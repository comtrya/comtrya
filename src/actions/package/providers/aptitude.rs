use super::PackageProvider;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aptitude {}

impl Aptitude {
    fn env(&self) -> Vec<(String, String)> {
        vec![(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        )]
    }
}

impl PackageProvider for Aptitude {
    fn name(&self) -> &str {
        "Aptitude"
    }

    fn available(&self) -> bool {
        match which("apt-add-repository") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "apt-add-repository not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("apt"),
                arguments: vec![
                    String::from("install"),
                    String::from("--yes"),
                    String::from("software-properties-common"),
                    String::from("curl"),
                    String::from("gpg"),
                ],
                environment: self.env(),
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
        let mut steps: Vec<Step> = vec![];

        if package.key.is_some() {
            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("bash"),
                    arguments: vec![
                        String::from("-c"),
                        String::from(format!(
                            "curl {} | apt-key add -",
                            package.key.clone().unwrap()
                        )),
                    ],
                    environment: self.env(),
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        steps.extend(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("apt-add-repository"),
                    arguments: vec![String::from("-y"), package.repository.clone().unwrap()],
                    environment: self.env(),
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("apt"),
                    arguments: vec![String::from("update")],
                    environment: self.env(),
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
                command: String::from("apt"),
                arguments: vec![String::from("install"), String::from("--yes")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                environment: self.env(),
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

    // These tests are really weak at the moment, but that's because I'm not
    // sure how to add derive(Debug,Default) to struct Step
    // TODO: Learn how to fix this

    #[test]
    fn test_add_repository_simple() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            ..Default::default()
        };

        let aptitude = Aptitude {};
        let steps = aptitude.add_repository(&package);

        let expected_steps: Vec<Step> = vec![];

        assert_eq!(steps.len(), expected_steps.len());
    }

    #[test]
    fn test_add_repository_without_key() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            repository: Some(String::from("repository")),
            ..Default::default()
        };

        let aptitude = Aptitude {};
        let steps = aptitude.add_repository(&package);

        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn test_add_repository_with_key() {
        let package = PackageVariant {
            name: Some(String::from("test")),
            repository: Some(String::from("repository")),
            key: Some(String::from("key")),
            ..Default::default()
        };

        let aptitude = Aptitude {};
        let steps = aptitude.add_repository(&package);

        assert_eq!(steps.len(), 3);
    }
}
