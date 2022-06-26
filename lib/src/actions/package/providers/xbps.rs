use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use tracing::{debug, trace, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Xbps {}

impl PackageProvider for Xbps {
    fn name(&self) -> &str {
        "Xbps"
    }

    fn available(&self) -> bool {
        let mut install = false;
        let mut query = false;
        match which("xbps-install") {
            Ok(_) => {
                install = true;
            }
            Err(_) => {
                warn!(message = "xbps-install not available");
            }
        };
        match which("xbps-query") {
            Ok(_) => {
                query = true;
            }
            Err(_) => {
                warn!(message = "xbps-query not available");
            }
        };
        install && query
    }

    fn bootstrap(&self) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository) -> Vec<Step> {
        vec![]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        let requested_already_installed: HashSet<String> = String::from_utf8(
            Command::new("xbps-query")
                .args(
                    vec![String::from("-s")]
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
        package
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
            .collect()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Step> {
        let need_installed = self.query(package);
        if need_installed.is_empty() {
            return vec![];
        }
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("xbps-install"),
                arguments: [
                    vec![
                        String::from("-S"),
                        String::from("--yes"),
                        String::from("--update"),
                    ],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
