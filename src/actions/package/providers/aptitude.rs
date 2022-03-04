use std::collections::HashMap;

use super::PackageProvider;
use crate::actions::{package::PackageVariant, ActionError};
use crate::utils::command::{run_command, Command};
use serde::{Deserialize, Serialize};
use tracing::{debug, span, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aptitude {}

impl Aptitude {
    fn env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        env.insert(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        );

        env
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

<<<<<<< Updated upstream
    fn bootstrap(&self) -> Result<(), crate::actions::ActionError> {
        // Apt should always be available on Debian / Ubuntu flavours.
        // Lets make sure software-properties-common is available
        // for repository management
        let span = span!(tracing::Level::INFO, "bootstrap").entered();

        run_command(Command {
            name: String::from("apt"),
            env: self.env(),
            dir: None,
            args: vec![
                String::from("install"),
                String::from("-y"),
                String::from("software-properties-common"),
                String::from("gpg"),
            ],
            require_root: true,
        })?;

        span.exit();

        Ok(())
=======
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
>>>>>>> Stashed changes
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

<<<<<<< Updated upstream
    fn add_repository(&self, package: &PackageVariant) -> Result<(), ActionError> {
        run_command(Command {
            name: String::from("apt-add-repository"),
            env: self.env(),
            dir: None,
            args: vec![String::from("-y"), package.repository.clone().unwrap()],
            require_root: true,
        })?;

        debug!(message = "Running Aptitude Update");

        run_command(Command {
            name: String::from("apt"),
            env: self.env(),
            dir: None,
            args: vec![String::from("update")],
            require_root: true,
        })?;

        Ok(())
=======
    fn add_repository(&self, package: &PackageVariant) -> Vec<Step> {
        if package.repository.is_none() {
            return vec![];
        }

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
>>>>>>> Stashed changes
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Result<(), ActionError> {
        run_command(Command {
            name: String::from("apt"),
            env: self.env(),
            dir: None,
            args: vec![String::from("install"), String::from("-y")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            require_root: true,
        })?;

        Ok(())
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
