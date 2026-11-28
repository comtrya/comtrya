//! pkgin on NetBSD, and on other systems with pkgsrc

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{instrument, warn};
use which::which;

/// pkgin. Tries the install with `pkgin -n` first, and stops when pkgin would remove packages
/// for it
#[derive(Clone)]
pub struct Pkgin {
    /// Runs `pkgin -p list` to find the installed and the outdated packages
    runner: Arc<dyn Runner>,
}

impl Default for Pkgin {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

/// Where pkgsrc puts pkgin
const PKGIN: &str = "/usr/pkg/bin/pkgin";

impl Pkgin {
    /// The installed packages `pkgin -p list` names, only those with the status flag `limit`
    /// when it's set. `<` means a newer version is in the repository summary pkgin has. Lines
    /// look like `git-2.47.0;Distributed version control system`
    fn listed(&self, limit: Option<&str>) -> Result<HashSet<String>> {
        let mut command = CommandSpec::new(PKGIN).arg("-p");
        if let Some(limit) = limit {
            command = command.args(["-l", limit]);
        }
        super::listed(&*self.runner, &command.arg("list"), |line| {
            let pkgver = line.split([';', ' ', '\t']).next()?;
            pkgver.rsplit_once('-').map(|(name, _version)| name)
        })
    }

    /// The requested packages that aren't installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let installed = self.listed(None)?;
        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// A dry run first, which stops when pkgin would remove packages, then `pkgin install`,
    /// which also upgrades installed packages it's given
    fn install_steps(extra_args: &[String], packages: &[String]) -> Vec<Step> {
        vec![
            // A dry run first: pkgin may remove packages to install others
            Step::new(Exec {
                command: String::from(PKGIN),
                arguments: [&["-n".into(), "install".into()], extra_args, packages].concat(),
                stop_if_stdout_contains: Some(String::from("removed")),
                ..Default::default()
            })
            .as_root(),
            Step::new(Exec {
                command: String::from(PKGIN),
                arguments: [&["-y".into(), "install".into()], extra_args, packages].concat(),
                ..Default::default()
            })
            .as_root(),
        ]
    }
}

impl PackageProvider for Pkgin {
    fn name(&self) -> &str {
        "Pkgin"
    }

    fn available(&self) -> bool {
        if which("/usr/pkg/bin/pkgin").is_ok() {
            true
        } else {
            warn!(message = "/usr/pkg/bin/pkgin is not available");
            false
        }
    }

    /// Installs pkgin with `pkg_add`, which comes with NetBSD
    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("/usr/sbin/pkg_add"),
            arguments: vec![String::from("pkgin")],
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    // TODO: Handle query pkgs with pkgin search
    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(Self::install_steps(
            &package.extra_args,
            &package.packages(),
        ))
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let installed = super::installed(package, &missing);
        let outdated = if installed.is_empty() {
            vec![]
        } else {
            super::among(&installed, &self.listed(Some("<"))?)
        };
        let targets: Vec<String> = package
            .packages()
            .into_iter()
            .filter(|p| missing.contains(p) || outdated.contains(p))
            .collect();
        if targets.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(Self::install_steps(&package.extra_args, &targets))
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.missing(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![Step::new(Exec {
            command: String::from(PKGIN),
            arguments: [vec![String::from("-y"), String::from("remove")], installed].concat(),
            ..Default::default()
        })
        .as_root()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::RunAs;
    use pretty_assertions::assert_eq;

    /// The first step only looks: a dry run, which stops the action if pkgin would remove
    /// packages. Only then pkgin installs
    #[test]
    fn install_stops_when_a_dry_run_would_remove_packages() {
        use crate::atoms::Atom;

        let package = PackageVariant {
            list: vec![String::from("git")],
            ..Default::default()
        };

        let steps = Pkgin::default()
            .install(&package, &Facts::default())
            .unwrap();

        let commands: Vec<String> = steps.iter().map(Step::to_string).collect();
        assert_eq!(
            vec![
                "Run /usr/pkg/bin/pkgin -n install git (as root)",
                "Run /usr/pkg/bin/pkgin -y install git (as root)",
            ],
            commands
        );
        let Atom::Exec(dry_run) = &steps[0].atom else {
            panic!("the dry run is a program");
        };
        assert_eq!(
            Some(String::from("removed")),
            dry_run.stop_if_stdout_contains
        );
    }

    #[test]
    fn bootstrap_installs_pkgin_with_pkg_add_as_root() {
        let steps = Pkgin::default().bootstrap(&Facts::default());

        assert_eq!(1, steps.len());
        assert_eq!(RunAs::Root, steps[0].run_as);
        assert_eq!(
            "Run /usr/sbin/pkg_add pkgin (as root)",
            steps[0].to_string()
        );
    }

    /// pkgin with git and vim installed, and `outdated` from `pkgin -p -l < list`
    fn with_outdated(outdated: &str) -> Pkgin {
        use crate::runner::fake::{succeeds, FakeRunner};

        Pkgin {
            runner: FakeRunner::new()
                .answer(
                    "/usr/pkg/bin/pkgin -p list",
                    succeeds("git-2.47.0;Distributed version control system\nvim-9.1.0836;Vim\n"),
                )
                .answer("/usr/pkg/bin/pkgin -p -l < list", succeeds(outdated))
                .shared(),
        }
    }

    fn packages(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: list.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    fn commands(steps: &[Step]) -> Vec<String> {
        steps.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn latest_installs_missing_and_outdated_packages() {
        let steps = with_outdated("vim-9.1.0836;Vim\n")
            .upgrade(&packages(&["git", "vim", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec![
                "Run /usr/pkg/bin/pkgin -n install vim ripgrep (as root)",
                "Run /usr/pkg/bin/pkgin -y install vim ripgrep (as root)",
            ],
            commands(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let steps = with_outdated("")
            .upgrade(&packages(&["git", "vim"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_packages() {
        let pkgin = with_outdated("");

        let steps = pkgin
            .remove(&packages(&["git", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = pkgin
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run /usr/pkg/bin/pkgin -y remove git (as root)"],
            commands(&steps)
        );
        assert!(nothing.is_empty());
    }
}
