//! pacman on Arch: packages from the official repositories, also as root

use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::actions::Result;
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// Packages from the official Arch repositories. AUR packages need `yay` or `paru`
#[derive(Clone)]
pub struct Pacman {
    /// Runs `pacman -Q` to find the installed packages
    runner: Arc<dyn Runner>,
}

impl Default for Pacman {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Pacman {
    fn name(&self) -> &str {
        "Pacman"
    }

    fn available(&self) -> bool {
        match which("pacman") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "pacman not available");
                false
            }
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _repository: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        missing(self.runner.as_ref(), package)
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![install_step(package, need_installed)])
    }
}

/// The requested packages that pacman doesn't list as installed. The AUR helpers use it too
pub(super) fn missing(runner: &dyn Runner, package: &PackageVariant) -> Result<Vec<String>> {
    // `pacman -Qq` prints the installed ones and fails for the others, so ignore the status
    let command = CommandSpec::new("pacman")
        .args(["-Q", "-q"])
        .args(package.packages());
    let installed: HashSet<String> =
        String::from_utf8_lossy(&super::query(runner, &command)?.stdout)
            .lines()
            .map(String::from)
            .collect();

    Ok(super::missing(package, |p| installed.contains(p)))
}

/// `pacman -S --needed` with the privilege provider
fn install_step(package: &PackageVariant, packages: Vec<String>) -> Step {
    Step::new(Exec {
        command: String::from("pacman"),
        arguments: [
            vec![
                String::from("-S"),
                String::from("--needed"),
                String::from("--noconfirm"),
            ],
            package.extra_args.clone(),
            packages,
        ]
        .concat(),
        ..Default::default()
    })
    .as_root()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::package::providers::PackageProviders;
    use crate::runner::fake::{fails, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_returns_the_packages_that_are_not_installed() {
        // pacman prints the installed ones, and fails because of the others
        let runner = FakeRunner::new()
            .answer(
                "pacman -Q -q git ripgrep",
                fails("git\n", "error: package 'ripgrep' was not found\n"),
            )
            .shared();
        let pacman = Pacman {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            list: vec![String::from("git"), String::from("ripgrep")],
            ..Default::default()
        };

        assert_eq!(vec!["ripgrep"], pacman.query(&package).unwrap());
        assert_eq!(vec!["pacman -Q -q git ripgrep"], runner.calls());
    }

    #[test]
    fn query_fails_without_pacman() {
        let pacman = Pacman {
            runner: FakeRunner::new().shared(),
        };
        let package = PackageVariant {
            name: Some(String::from("git")),
            ..Default::default()
        };

        assert!(pacman.query(&package).is_err());
    }

    #[test]
    fn installs_with_privileges() {
        let package = PackageVariant {
            name: None,
            list: vec![String::from("git"), String::from("ripgrep")],
            extra_args: vec![String::from("--asdeps")],
            provider: Some(PackageProviders::Pacman),
            file: false,
        };

        let step = install_step(&package, package.packages());

        assert_eq!(
            "CommandExec with: pacman -S --needed --noconfirm --asdeps git ripgrep (as root)",
            step.to_string()
        );
    }
}
