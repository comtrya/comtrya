use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Macports {}

impl PackageProvider for Macports {
    fn name(&self) -> &str {
        "MacPorts"
    }

    fn available(&self) -> bool {
        match which("port") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "MacPorts not available, check if in $PATH");
                false
            }
        }
    }

    fn bootstrap(&self) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, _repository: &PackageRepository) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, _package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }

    fn install(&self, package: &PackageVariant) -> anyhow::Result<Vec<Step>> {
        let cli = match which("port") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "MacPorts is not availiable.");
                return Ok(vec![]);
            }
        };

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
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
        }])
    }
}
