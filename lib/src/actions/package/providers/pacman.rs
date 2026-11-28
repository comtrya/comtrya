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
        if which("pacman").is_ok() {
            true
        } else {
            warn!(message = "pacman not available");
            false
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

        // The executor runs the step before it plans the next action, which reads the list again
        self.runner.forget();
        Ok(vec![install_step(package, need_installed)])
    }

    /// `pacman -S --needed` installs the missing packages and upgrades the outdated ones alike
    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let targets = missing_or_outdated(self.runner.as_ref(), package, &[])?;
        if targets.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![install_step(package, targets)])
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.query(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![Step::new(Exec {
            command: String::from("pacman"),
            arguments: [
                vec![String::from("-R"), String::from("--noconfirm")],
                installed,
            ]
            .concat(),
            ..Default::default()
        })
        .as_root()])
    }
}

/// The requested packages that are missing, or that pacman (`pacman -Q -u`) or one of the
/// `also_outdated` commands lists as outdated, in the order they were requested. The AUR
/// helpers use it too, with their own list of outdated AUR packages
pub(super) fn missing_or_outdated(
    runner: &dyn Runner,
    package: &PackageVariant,
    also_outdated: &[CommandSpec],
) -> Result<Vec<String>> {
    let missing = missing(runner, package)?;
    if missing.len() == package.packages().len() {
        return Ok(missing);
    }

    // `-Q -u` lists nothing and fails when everything is up to date
    let mut outdated = HashSet::new();
    for command in std::iter::once(CommandSpec::new("pacman").args(["-Q", "-u", "-q"]))
        .chain(also_outdated.iter().cloned())
    {
        outdated.extend(super::listed(runner, &command, |line| {
            line.split_whitespace().next()
        })?);
    }

    Ok(package
        .packages()
        .into_iter()
        .filter(|p| missing.contains(p) || outdated.contains(p))
        .collect())
}

/// The requested packages that pacman doesn't list as installed. The AUR helpers use it too.
/// The list is the same for every package, so the shared runner reads it once per run
pub(super) fn missing(runner: &dyn Runner, package: &PackageVariant) -> Result<Vec<String>> {
    // A failing `pacman -Qq` lists nothing, then everything is installed with `--needed`
    let command = CommandSpec::new("pacman").args(["-Q", "-q"]);
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
    use crate::runner::fake::{succeeds, FakeRunner};
    use crate::runner::Cached;
    use pretty_assertions::assert_eq;

    #[test]
    fn query_returns_the_packages_that_are_not_installed() {
        let runner = FakeRunner::new()
            .answer("pacman -Q -q", succeeds("git\nvim\n"))
            .shared();
        let pacman = Pacman {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            list: vec![String::from("git"), String::from("ripgrep")],
            ..Default::default()
        };

        assert_eq!(vec!["ripgrep"], pacman.query(&package).unwrap());
        assert_eq!(vec!["pacman -Q -q"], runner.calls());
    }

    #[test]
    fn two_actions_read_the_installed_packages_once() {
        let fake = FakeRunner::new()
            .answer("pacman -Q -q", succeeds("git\nripgrep\n"))
            .shared();
        let pacman = Pacman {
            runner: Arc::new(Cached::new(fake.clone())),
        };

        for name in ["git", "ripgrep"] {
            let package = PackageVariant {
                name: Some(String::from(name)),
                ..Default::default()
            };
            assert!(pacman
                .install(&package, &Facts::default())
                .unwrap()
                .is_empty());
        }

        assert_eq!(vec!["pacman -Q -q"], fake.calls());
    }

    #[test]
    fn packages_are_read_again_after_an_install() {
        let fake = FakeRunner::new()
            .answer("pacman -Q -q", succeeds("git\n"))
            .shared();
        let pacman = Pacman {
            runner: Arc::new(Cached::new(fake.clone())),
        };
        let package = PackageVariant {
            name: Some(String::from("ripgrep")),
            ..Default::default()
        };

        // The executor runs the install before it plans the next action
        assert_eq!(
            1,
            pacman.install(&package, &Facts::default()).unwrap().len()
        );
        pacman.query(&package).unwrap();

        assert_eq!(vec!["pacman -Q -q", "pacman -Q -q"], fake.calls());
    }

    fn packages(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: list.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn latest_installs_missing_and_outdated_packages_in_one_step() {
        let pacman = Pacman {
            runner: FakeRunner::new()
                .answer("pacman -Q -q", succeeds("git\nvim\nzsh\n"))
                .answer("pacman -Q -u -q", succeeds("vim\nlinux\n"))
                .shared(),
        };

        let steps = pacman
            .upgrade(&packages(&["git", "vim", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(1, steps.len());
        assert_eq!(
            "Run pacman -S --needed --noconfirm vim ripgrep (as root)",
            steps[0].to_string()
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let pacman = Pacman {
            runner: FakeRunner::new()
                .answer("pacman -Q -q", succeeds("git\nvim\n"))
                .answer("pacman -Q -u -q", crate::runner::fake::fails("", ""))
                .shared(),
        };

        let steps = pacman
            .upgrade(&packages(&["git", "vim"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_packages_as_root() {
        let pacman = Pacman {
            runner: FakeRunner::new()
                .answer("pacman -Q -q", succeeds("git\nvim\n"))
                .shared(),
        };

        let steps = pacman
            .remove(&packages(&["vim", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = pacman
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(1, steps.len());
        assert_eq!(
            "Run pacman -R --noconfirm vim (as root)",
            steps[0].to_string()
        );
        assert!(nothing.is_empty());
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
            "Run pacman -S --needed --noconfirm --asdeps git ripgrep (as root)",
            step.to_string()
        );
    }
}
