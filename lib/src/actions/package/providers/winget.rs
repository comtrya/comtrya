//! winget on Windows

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::contexts::Contexts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::sync::Arc;
use tracing::warn;
use which::which;

/// winget. Installs one package per step, from the `winget` source
#[derive(Clone)]
pub struct Winget {
    /// Runs `winget list` to find the installed packages
    runner: Arc<dyn Runner>,
}

impl Default for Winget {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Winget {
    /// Whether `winget list` finds the package. A failing winget counts as not installed, so
    /// the install is tried anyway
    fn is_installed(&self, package: &str) -> bool {
        let output = match self
            .runner
            .run(&CommandSpec::new("winget").args(list_args(package)))
        {
            Ok(output) => output,
            Err(e) => {
                warn!("Failed to query winget for package {}: {}", package, e);
                return false;
            }
        };

        // winget exits with 0 even if nothing is found, and prints "No installed package found"
        String::from_utf8_lossy(&output.stdout)
            .to_lowercase()
            .contains(&package.to_lowercase())
    }
}

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
        Ok(super::missing(package, |p| self.is_installed(p)))
    }

    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // does not require privilege escalation

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(need_installed
            .iter()
            .map::<Step, _>(|p| {
                Step::new(Exec {
                    command: String::from("winget"),
                    arguments: install_args(&package.extra_args, p),
                    ..Default::default()
                })
            })
            .collect())
    }
}

/// `winget list -e --id <package>`. `--accept-source-agreements` keeps it from blocking on the
/// first run, `--disable-interactivity` stops it from prompting at all
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

/// `winget install` for one package, without any prompts
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
    use crate::runner::fake::{succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_reads_the_list_output_because_the_exit_code_is_always_0() {
        let list = |id: &str| format!("winget {}", list_args(id).join(" "));
        let winget = Winget {
            runner: FakeRunner::new()
                .answer(
                    &list("Git.Git"),
                    succeeds("Name Id      Version\nGit  Git.Git 2.47.0\n"),
                )
                .answer(
                    &list("BurntSushi.ripgrep.MSVC"),
                    succeeds("No installed package found matching input criteria.\n"),
                )
                .shared(),
        };
        let package = PackageVariant {
            list: vec![
                String::from("Git.Git"),
                String::from("BurntSushi.ripgrep.MSVC"),
                // winget fails to run: assumed missing, so it's installed
                String::from("Microsoft.PowerShell"),
            ],
            ..Default::default()
        };

        assert_eq!(
            vec!["BurntSushi.ripgrep.MSVC", "Microsoft.PowerShell"],
            winget.query(&package).unwrap()
        );
    }

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
