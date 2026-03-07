use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, trace, warn};
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

    fn bootstrap(&self, _contexts: &Contexts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        true
    }

    fn add_repository(
        &self,
        _: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        // Find all packages that aren't already installed
        Ok(package
            .packages()
            .into_iter()
            .filter(|p| {
                // We use `winget list -e --id <package_name>`
                // `--accept-source-agreements` prevents it from blocking on first run
                let output = Command::new("winget")
                    .args([
                        "list",
                        "-e",
                        "--id",
                        p.as_str(),
                        "--accept-source-agreements",
                    ])
                    .output();
                
                match output {
                    Ok(output) => {
                        // Winget returns 0 if found, non-zero if not found or error
                        if output.status.success() {
                            trace!("{}: already installed", p);
                            false // installed, so filter it out
                        } else {
                            debug!("{}: doesn't appear to be installed", p);
                            true // not installed, keep it
                        }
                    }
                    Err(e) => {
                        warn!("Failed to query winget for package {}: {}", p, e);
                        true // assume not installed on error to attempt install
                    }
                }
            })
            .collect())
    }

    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // does not require privilege escalation

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(need_installed
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
