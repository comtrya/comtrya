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
                // `--accept-source-agreements` prevents it from blocking on first run,
                // `--disable-interactivity` stops winget from prompting at all
                let output = Command::new("winget").args(list_args(p)).output();

                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // Winget returns 0 even if nothing is found, so we must parse the output.
                        // If it contains the package ID, it's installed. Otherwise, it prints "No installed package found"
                        if stdout.to_lowercase().contains(&p.to_lowercase()) {
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
                    arguments: install_args(&package.extra_args, p),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect())
    }
}

fn list_args(package: &str) -> Vec<String> {
    [
        "list",
        "-e",
        "--id",
        package,
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
    .map(String::from)
    .to_vec()
}

fn install_args(extra_args: &[String], package: &str) -> Vec<String> {
    [
        vec![
            "install".to_string(),
            "--silent".to_string(),
            "--accept-package-agreements".to_string(),
            "--accept-source-agreements".to_string(),
            "--disable-interactivity".to_string(),
            "--source".to_string(),
            "winget".to_string(),
        ],
        extra_args.to_vec(),
        vec![package.to_string()],
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // winget must never wait for input, comtrya runs unattended
    const NON_INTERACTIVE: [&str; 2] = ["--accept-source-agreements", "--disable-interactivity"];

    #[test]
    fn list_is_non_interactive() {
        let args = list_args("Git.Git");

        assert_eq!(vec!["list", "-e", "--id", "Git.Git"], args[..4]);
        for flag in NON_INTERACTIVE {
            assert!(args.contains(&flag.to_string()), "missing {flag}");
        }
    }

    #[test]
    fn install_is_silent_and_non_interactive() {
        let args = install_args(&[], "Git.Git");

        assert_eq!("install", args[0]);
        for flag in NON_INTERACTIVE
            .iter()
            .chain(&["--silent", "--accept-package-agreements"])
        {
            assert!(args.contains(&flag.to_string()), "missing {flag}");
        }
    }

    #[test]
    fn install_puts_extra_args_before_the_package() {
        let args = install_args(&["--scope".to_string(), "user".to_string()], "Git.Git");

        assert_eq!(vec!["--scope", "user", "Git.Git"], args[args.len() - 3..]);
    }
}
