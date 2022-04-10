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

    // TODO: What is the desired state for query(): to be package.packages() or to write custom code for each manager(like yay.rs and homebrew.rs, but not aptitude.rs)?
    // I tried to implement query() for dnf by getting all installed packages "dnf list installed | awk '{print $1}'" and checking if they existed in package.packages,
    // but I'm not entirely sure what the goal of query() is so for now I've left it as package.packages() and left the commented out code below
    fn query(&self, package: &PackageVariant) -> Vec<String> {
        package.packages()
    }

    // TODO: remove this commented out code, leaving here for now so reviewer can read
    /*
    fn query(&self, package: &PackageVariant) -> Vec<String> {
        let requested_already_installed: HashSet<String> = String::from_utf8(
            // dnf unfortunately doesn't have a package-name-only list option,
            // so I used awk here to only print the package-names https://unix.stackexchange.com/questions/698003/how-to-tell-dnf-search-to-list-only-matches-in-the-pacakge-name-or-name-and-s
            // TODO: should this below command be getting all installed packages?? Or is it meant to get dependencies of the package?
            Command::new("dnf")
                .args(vec![
                    String::from("list"),
                    String::from("installed"),
                    // TODO: any better alternative to bash pipe or awk here?
                    String::from("|"),
                    String::from("awk"),
                    String::from("'{print $1}'"),
                ])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .sprint('\n')
        .map(String::from)
        .collect();

        debug!(
            "all requested installed packages: {:?}",
            requested_already_installed
        );
        package
            .packages()
            .into_iter()
            .filter(|p| {
                if requested_already_installed.contains(p) {
                    trace!("{}: already installed", p);
                    false
                } else {
                    debug!("{}: doesn't appear to bew installed", p);
                    true
                }
            })
            .collect()
    }
    */

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
                // TODO: OK to call self.query(package) directly here?
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

    // TODO: Same as aptitude.rs, weak tests that should be updated in future

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
