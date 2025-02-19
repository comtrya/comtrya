use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::steps::finalizers::FlowControl::StopIf;
use crate::steps::finalizers::OutputContains;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec, utilities};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        vec![Step {
            atom: Box::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("bootstrap")],
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
        _: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        if package.file {
            return Ok(vec![Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/sbin/pkg"),
                    arguments: vec![String::from("add")]
                        .into_iter()
                        .chain(package.extra_args.clone())
                        .chain(package.packages())
                        .collect(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            }]);
        }

        Ok(vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/sbin/pkg"),
                    arguments: vec![String::from("install"), String::from("-y")]
                        .into_iter()
                        .chain(package.extra_args.clone())
                        .chain(package.packages())
                        .collect(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![StopIf(Box::new(OutputContains("removed")))],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/sbin/pkg"),
                    arguments: vec![String::from("install"), String::from("-y")]
                        .into_iter()
                        .chain(package.extra_args.clone())
                        .chain(package.packages())
                        .collect(),
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ])
    }
}
