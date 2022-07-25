use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use sha256::digest;
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zypper {}

impl PackageProvider for Zypper {
    fn name(&self) -> &str {
        "Zypper"
    }

    fn available(&self) -> bool {
        match which("zypper") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "zypper not availiable");
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

    fn add_repository(&self, _repository: &PackageRepository) -> Vec<Step> {
	vec![]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Step> {
        vec![Step {
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
        }]
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::{providers::PackageProviders};

    use super::*;

    // These tests are really weak at the moment, but that's because I'm not
    // sure how to add derive(Debug,Default) to struct Step
    // TODO: Learn how to fix this

    // #[test]
    // fn test_add_repository_with_key_and_fingerprint() {
    //     let aptitude = Aptitude {};
    //     let steps = aptitude.add_repository(&PackageRepository {
    //         name: String::from("test"),
    //         key: Some(RepositoryKey {
    //             url: String::from("abc"),
    //             fingerprint: Some(String::from("abc")),
    //             ..Default::default()
    //         }),
    //         ..Default::default()
    //     });

    //     assert_eq!(steps.len(), 3);
    // }

    #[test]
    fn test_install() {
	    let zypper = Zypper {};
	    let steps = zypper.install(&PackageVariant {
            name: Some(String::from("")),
            list: vec![],
            extra_args: vec![],
            provider: PackageProviders::Zypper,
        });

        assert_eq!(steps.len(), 1);
    }
}
