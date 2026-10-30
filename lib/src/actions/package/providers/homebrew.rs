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
            let name = p.rsplit('/').next().unwrap_or(p);
            cellar.join(name).is_dir() || caskroom.join(name).is_dir()
        }))
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        // Does not require privilege escalation

        let need_installed = self.query(package)?;

        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step::new(Exec {
            command: String::from("brew"),
            arguments: [
                vec![String::from("install")],
                package.extra_args.clone(),
                need_installed,
            ]
            .concat(),
            ..Default::default()
        })])
    }
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
