use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use serde::{Deserialize, Serialize};
use sha256::digest;
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

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
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        repository: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        let mut signed_by = String::from("");

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        if repository.key.is_some() {
            // .unwrap() is safe here because we checked for key.is_some() above
            let key = repository.clone().key.unwrap();

            let key_name = key.name.unwrap_or_else(|| digest(&*key.url));
            let key_path = format!("/usr/share/keyrings/{key_name}.asc");

            signed_by = format!("signed-by={key_path}");

            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("curl"),
                    arguments: vec![String::from("-o"), key_path, key.url],
                    environment: self.env(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        //sudo apt-add-repository "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/<myrepository>-archive-keyring.gpg] https://repository.example.com/debian/ $(lsb_release -cs) stable main "
        steps.extend(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("apt-add-repository"),
                    arguments: vec![
                        String::from("-y"),
                        format!(
                            "deb [arch=$(dpkg --print-architecture) {}] {}",
                            signed_by,
                            repository.name.clone()
                        ),
                    ],
                    environment: self.env(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
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
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]);

        Ok(steps)
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("apt"),
                arguments: vec![String::from("install"), String::from("--yes")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                environment: self.env(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::repository::RepositoryKey;
    use crate::contexts::Contexts;

    use super::*;

    // These tests are really weak at the moment, but that's because I'm not
    // sure how to add derive(Debug,Default) to struct Step
    // TODO: Learn how to fix this

    #[test]
    fn test_add_repository_without_key() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_add_repository_with_key() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }

    #[test]
    fn test_add_repository_with_key_and_fingerprint() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    fingerprint: Some(String::from("abc")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }

    #[test]
    fn test_regression_share_ring() {
        let aptitude = Aptitude {};
        let contexts = Contexts::default();
        let steps = aptitude.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    fingerprint: Some(String::from("abc")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            &contexts,
        );

        let steps = steps.unwrap_or_default();

        if let Some(step) = steps.first() {
            let exec = step.atom.to_string();
            assert!(exec.contains(" /usr/share/keyrings/"));
        } else {
            panic!("expected at least one step");
        }
    }
}
