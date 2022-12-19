use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Winget {}

impl PackageProvider for Winget {
    fn name(&self) -> &str {
        "Winget"
    }

    fn available(&self) -> bool {
        match which("winget") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "winget not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        true
    }

    fn add_repository(&self, _: &PackageRepository) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        // Install all packages, make this smarter soon
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant) -> anyhow::Result<Vec<Step>> {
        Ok(package
            .packages()
            .iter()
            .map::<Step, _>(|p| Step {
                atom: Box::new(Exec {
                    command: String::from("winget"),
                    arguments: [
                        vec![
                            "install".to_string(),
                            "--silent".to_string(),
                            "--accept-package-agreements".to_string(),
                            "--accept-source-agreements".to_string(),
                            "--source".to_string(),
                            "winget".to_string(),
                        ],
                        package.extra_args.clone(),
                        vec![p.clone()],
                    ]
                    .concat(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect())
    }
}
