//! Homebrew on macOS. comtrya installs Homebrew itself when it's missing

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::{path::Path, sync::Arc};
use which::which;

/// Homebrew, as `homebrew` or `brew`. Repositories are taps
#[derive(Clone)]
pub struct Homebrew {
    /// Runs `brew --prefix` to find the installed formulae and casks
    runner: Arc<dyn Runner>,
}

impl Default for Homebrew {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Homebrew {
    fn name(&self) -> &str {
        "Homebrew"
    }

    fn available(&self) -> bool {
        which("brew").is_ok()
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("bash"),
            arguments: vec![
                String::from("-c"),
                String::from("$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)")
            ],
            ..Default::default()
        }),]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, repository: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("brew"),
            arguments: vec![String::from("tap"), repository.name.clone()],
            ..Default::default()
        })])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let output = super::query(&*self.runner, &CommandSpec::new("brew").arg("--prefix"))?;
        let prefix = String::from_utf8_lossy(&output.stdout).replace(['\n', '\r'], "");

        let cellar = Path::new(&prefix).join("Cellar");
        let caskroom = Path::new(&prefix).join("Caskroom");

        // Formulae are installed below the Cellar, casks below the Caskroom, both by their name
        // without the tap, like `terraform` for `hashicorp/tap/terraform`
        Ok(super::missing(package, |p| {
            let name = short_name(p);
            cellar.join(name).is_dir() || caskroom.join(name).is_dir()
        }))
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        // Does not require privilege escalation

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![brew("install", &package.extra_args, need_installed)])
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.query(package)?;
        let outdated = self.outdated(&super::installed(package, &missing))?;

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.push(brew("install", &package.extra_args, missing));
        }
        if !outdated.is_empty() {
            // The executor runs the step before it plans the next action, which asks again
            self.runner.forget();
            steps.push(brew("upgrade", &package.extra_args, outdated));
        }

        Ok(steps)
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.query(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![brew("uninstall", &[], installed)])
    }
}

impl Homebrew {
    /// The installed packages that `brew outdated` lists. It names formulae and casks without
    /// their tap, so they're compared by the name after the last `/`
    fn outdated(&self, installed: &[String]) -> Result<Vec<String>> {
        if installed.is_empty() {
            return Ok(vec![]);
        }

        let command = CommandSpec::new("brew").args(["outdated", "--quiet"]);
        let outdated = super::listed(&*self.runner, &command, |line| Some(short_name(line)))?;

        Ok(installed
            .iter()
            .filter(|p| outdated.contains(short_name(p)))
            .cloned()
            .collect())
    }
}

/// The name of a formula or cask without its tap, like `terraform` for `hashicorp/tap/terraform`
fn short_name(package: &str) -> &str {
    package.rsplit('/').next().unwrap_or(package)
}

/// `brew <subcommand>` for the packages, as the user
fn brew(subcommand: &str, extra_args: &[String], packages: Vec<String>) -> Step {
    Step::new(Exec {
        command: String::from("brew"),
        arguments: [
            vec![String::from(subcommand)],
            extra_args.to_vec(),
            packages,
        ]
        .concat(),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_looks_for_formulae_and_casks_below_the_prefix() {
        let prefix = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(prefix.path().join("Cellar/git")).unwrap();
        std::fs::create_dir_all(prefix.path().join("Caskroom/firefox")).unwrap();
        let homebrew = Homebrew {
            runner: FakeRunner::new()
                .answer(
                    "brew --prefix",
                    succeeds(&format!("{}\n", prefix.path().display())),
                )
                .shared(),
        };
        let package = PackageVariant {
            list: vec![
                String::from("git"),
                String::from("firefox"),
                String::from("ripgrep"),
            ],
            ..Default::default()
        };

        assert_eq!(vec!["ripgrep"], homebrew.query(&package).unwrap());
    }

    /// Homebrew with `git` and `firefox` installed below a temporary prefix, and `brew outdated`
    /// answering `outdated`
    fn installed_git_and_firefox(prefix: &Path, outdated: &str) -> (Homebrew, Arc<FakeRunner>) {
        std::fs::create_dir_all(prefix.join("Cellar/git")).unwrap();
        std::fs::create_dir_all(prefix.join("Caskroom/firefox")).unwrap();
        let runner = FakeRunner::new()
            .answer(
                "brew --prefix",
                succeeds(&format!("{}\n", prefix.display())),
            )
            .answer("brew outdated --quiet", succeeds(outdated))
            .shared();

        (
            Homebrew {
                runner: runner.clone(),
            },
            runner,
        )
    }

    fn steps(steps: Vec<Step>) -> Vec<String> {
        steps.iter().map(ToString::to_string).collect()
    }

    fn packages(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: list.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn latest_installs_missing_and_upgrades_outdated_packages() {
        let prefix = tempfile::tempdir().unwrap();
        let (homebrew, _) = installed_git_and_firefox(prefix.path(), "git\nnode\n");

        let planned = homebrew
            .upgrade(&packages(&["git", "firefox", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run brew install ripgrep", "Run brew upgrade git"],
            steps(planned)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let prefix = tempfile::tempdir().unwrap();
        let (homebrew, _) = installed_git_and_firefox(prefix.path(), "node\n");

        let planned = homebrew
            .upgrade(
                &packages(&["git", "homebrew/cask/firefox"]),
                &Facts::default(),
            )
            .unwrap();

        assert!(planned.is_empty());
    }

    #[test]
    fn latest_compares_tap_packages_by_their_name() {
        let prefix = tempfile::tempdir().unwrap();
        let (homebrew, _) = installed_git_and_firefox(prefix.path(), "firefox\n");

        let planned = homebrew
            .upgrade(&packages(&["homebrew/cask/firefox"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run brew upgrade homebrew/cask/firefox"],
            steps(planned)
        );
    }

    #[test]
    fn latest_doesnt_ask_for_outdated_packages_when_none_is_installed() {
        let prefix = tempfile::tempdir().unwrap();
        let (homebrew, runner) = installed_git_and_firefox(prefix.path(), "");

        let planned = homebrew
            .upgrade(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["Run brew install ripgrep"], steps(planned));
        assert_eq!(vec!["brew --prefix"], runner.calls());
    }

    #[test]
    fn absent_removes_only_installed_packages() {
        let prefix = tempfile::tempdir().unwrap();
        let (homebrew, _) = installed_git_and_firefox(prefix.path(), "");

        let planned = homebrew
            .remove(&packages(&["git", "ripgrep", "firefox"]), &Facts::default())
            .unwrap();
        let nothing = homebrew
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["Run brew uninstall git firefox"], steps(planned));
        assert!(nothing.is_empty());
    }

    #[test]
    fn query_finds_formulae_from_taps_by_their_name() {
        let prefix = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(prefix.path().join("Cellar/terraform")).unwrap();
        std::fs::create_dir_all(prefix.path().join("Caskroom/firefox")).unwrap();
        let homebrew = Homebrew {
            runner: FakeRunner::new()
                .answer(
                    "brew --prefix",
                    succeeds(&format!("{}\n", prefix.path().display())),
                )
                .shared(),
        };
        let package = PackageVariant {
            list: vec![
                String::from("hashicorp/tap/terraform"),
                String::from("homebrew/cask/firefox"),
                String::from("hashicorp/tap/vault"),
            ],
            ..Default::default()
        };

        assert_eq!(
            vec!["hashicorp/tap/vault"],
            homebrew.query(&package).unwrap()
        );
    }
}
