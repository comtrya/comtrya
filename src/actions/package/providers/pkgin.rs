use super::PackageProvider;
use crate::steps::finalizers::FlowControl::StopIf;
use crate::steps::finalizers::OutputContains;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;
// use os_info;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pkgin {}

impl Pkgin {
    fn env(&self) -> Vec<(String, String)> {
        vec![(String::from("ASSUME_ALWAYS_YES"), String::from("true"))]
    }
}

impl PackageProvider for Pkgin {
    fn name(&self) -> &str {
        "Pkgin"
    }

    fn available(&self) -> bool {
        match which("/usr/pkg/bin/pkgin") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "/usr/local/sbin/pkg is not available");
                false
            }
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("bootstrap")],
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

    fn add_repository(&self, _package: &PackageVariant) -> Vec<Step> {
        vec![]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages for now, don't get smart about which
        // already are
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Step> {
        vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/pkg/bin/pkgin"),
                    arguments: vec![String::from("install"), String::from("-n")]
                        .into_iter()
                        .chain(package.extra_args.clone())
                        .chain(package.packages())
                        .collect(),
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![StopIf(Box::new(OutputContains("removed")))],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/pkg/bin/pkgin"),
                    arguments: vec![String::from("install")]
                        .into_iter()
                        .chain(package.extra_args.clone())
                        .chain(package.packages())
                        .collect(),
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]
    }
}
