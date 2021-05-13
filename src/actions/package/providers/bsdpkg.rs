use super::PackageProvider;
use crate::atoms::command::finalizers::output_contains::OutputContains;
use crate::atoms::command::finalizers::FlowControl::FinishIf;
use crate::{
    actions::package::PackageVariant,
    atoms::{command::Exec, Atom},
};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BsdPkg {}

impl BsdPkg {
    fn env(&self) -> Vec<(String, String)> {
        vec![(String::from("ASSUME_ALWAYS_YES"), String::from("true"))]
    }
}

impl PackageProvider for BsdPkg {
    fn name(&self) -> &str {
        "BsdPkg"
    }

    fn available(&self) -> bool {
        match which("/usr/local/sbin/pkg") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "/usr/local/sbin/pkg is not available");
                false
            }
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self) -> Vec<Box<dyn Atom>> {
        vec![Box::new(Exec {
            command: String::from("/usr/sbin/pkg"),
            arguments: vec![String::from("bootstrap")],
            environment: self.env(),
            privileged: true,
            ..Default::default()
        })]
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        false
    }

    fn add_repository(&self, _package: &PackageVariant) -> Vec<Box<dyn Atom>> {
        vec![]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages for now, don't get smart about which
        // already are
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Box<dyn Atom>> {
        vec![
            Box::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("install"), String::from("-n")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                finalizers: vec![FinishIf(Box::new(OutputContains("removed")))],
                privileged: true,
                ..Default::default()
            }),
            Box::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("install")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                privileged: true,
                ..Default::default()
            }),
        ]
    }
}
