use super::PackageProvider;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use tracing::warn;
use tracing::{debug, trace};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Yay {}

impl PackageProvider for Yay {
    fn name(&self) -> &str {
        "Yay"
    }

    fn available(&self) -> bool {
        match which("yay") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "yay not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Option<Vec<Step>> {
        Some(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("pacman"),
                    arguments: vec![
                        String::from("-S"),
                        String::from("--noconfirm"),
                        String::from("base-devel"),
                        String::from("git"),
                    ],
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("git"),
                    arguments: vec![
                        String::from("clone"),
                        String::from("https://aur.archlinux.org/yay.git"),
                        String::from("/tmp/yay"),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("makepkg"),
                    arguments: vec![String::from("-si"), String::from("--noconfirm")],
                    working_dir: Some(String::from("/tmp/yay")),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ])
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, _package: &PackageVariant) -> Option<Vec<Step>> {
        // vec![]
        None
    }

    fn query(&self, package: &PackageVariant) -> Option<Vec<String>> {
        let requested_already_installed: HashSet<String> = String::from_utf8(
            Command::new("yay")
                .args(
                    vec![String::from("-Q"), String::from("-q")]
                        .into_iter()
                        .chain(package.packages().into_iter()),
                )
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .split('\n')
        .map(String::from)
        .collect();

        debug!(
            "all requested installed packages: {:?}",
            requested_already_installed
        );
        Some(package
            .packages()
            .into_iter()
            .filter(|p| {
                if requested_already_installed.contains(p) {
                    trace!("{}: already installed", p);
                    false
                } else {
                    debug!("{}: doesn't appear to be installed", p);
                    true
                }
            })
            .collect())
    }

    fn install(&self, package: &PackageVariant) -> Option<Vec<Step>> {
        // let need_installed = self.query(package);

        //if need_installed.unwrap().is_empty() {
            // return None;
        //}

        let need_installed = match self.query(package) {
            Some(packages) => packages,
            None => return None, 
        };

        Some(vec![Step {
            atom: Box::new(Exec {
                command: String::from("yay"),
                arguments: [
                    vec![
                        String::from("-S"),
                        String::from("--noconfirm"),
                        String::from("--nocleanmenu"),
                        String::from("--nodiffmenu"),
                    ],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
