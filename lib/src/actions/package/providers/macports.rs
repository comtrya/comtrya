//! MacPorts on macOS

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// MacPorts, as `macports` or `port`. Does nothing when `port` isn't installed
#[derive(Clone)]
pub struct Macports {
    /// Runs `port -q installed` and `port -q outdated` to find the installed and the outdated
    /// ports
    runner: Arc<dyn Runner>,
}

impl Default for Macports {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Macports {
    /// The ports `port -q <query>` lists, by the first word of each line, like
    /// `git @2.47.0_0+diff_highlight (active)` or `git 2.47.0_0 < 2.47.1_0`
    fn listed(&self, query: &str) -> Result<HashSet<String>> {
        let command = CommandSpec::new("port").args(["-q", query]);
        super::listed(&*self.runner, &command, |line| {
            line.split_whitespace().next()
        })
    }

    /// The requested ports that aren't installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let installed = self.listed("installed")?;
        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// `port <subcommand> <args> <ports>`, as root. `port` by its full path, which root's
    /// `PATH` usually doesn't have
    fn port(subcommand: &str, extra_args: &[String], ports: Vec<String>) -> Step {
        let cli =
            which("port").map_or_else(|_| String::from("port"), |cli| cli.display().to_string());
        Step::new(Exec {
            command: cli,
            arguments: [vec![String::from(subcommand)], extra_args.to_vec(), ports].concat(),
            ..Default::default()
        })
        .as_root()
    }
}

impl PackageProvider for Macports {
    fn name(&self) -> &str {
        "MacPorts"
    }

    fn available(&self) -> bool {
        if which("port").is_ok() {
            true
        } else {
            warn!(message = "MacPorts not available, check if in $PATH");
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

    fn query(&self, _package: &PackageVariant) -> Result<Vec<String>> {
        Ok(vec![])
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        if which("port").is_err() {
            warn!(message = "MacPorts is not availiable.");
            return Ok(vec![]);
        }

        Ok(vec![Self::port(
            "install",
            &package.extra_args,
            package.packages(),
        )])
    }

    /// Outdated ports are found in the port tree MacPorts has; `port selfupdate` refreshes it
    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let installed = super::installed(package, &missing);
        let outdated = if installed.is_empty() {
            vec![]
        } else {
            super::among(&installed, &self.listed("outdated")?)
        };

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.push(Self::port("install", &package.extra_args, missing));
        }
        if !outdated.is_empty() {
            steps.push(Self::port("upgrade", &package.extra_args, outdated));
        }
        if !steps.is_empty() {
            // The executor runs the steps before it plans the next action, which asks again
            self.runner.forget();
        }
        Ok(steps)
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.missing(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![Self::port("uninstall", &[], installed)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Atom;
    use crate::runner::fake::{succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    /// MacPorts with git and vim installed, and `outdated` from `port -q outdated`
    fn with_outdated(outdated: &str) -> Macports {
        Macports {
            runner: FakeRunner::new()
                .answer(
                    "port -q installed",
                    succeeds(
                        "  git @2.47.0_0+diff_highlight (active)\n  vim @9.1.0866_0 (active)\n",
                    ),
                )
                .answer("port -q outdated", succeeds(outdated))
                .shared(),
        }
    }

    fn packages(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: list.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }
    }

    /// The arguments of the steps, without the path of `port`, which differs per machine
    fn commands(steps: &[Step]) -> Vec<String> {
        steps
            .iter()
            .map(|step| {
                let Atom::Exec(exec) = &step.atom else {
                    panic!("a port step is a program");
                };
                exec.arguments.join(" ")
            })
            .collect()
    }

    #[test]
    fn latest_installs_missing_and_upgrades_outdated_ports() {
        let steps = with_outdated("git                            2.47.0_0 < 2.47.1_0\n")
            .upgrade(&packages(&["git", "vim", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["install ripgrep", "upgrade git"], commands(&steps));
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let steps = with_outdated("")
            .upgrade(&packages(&["git", "vim"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_uninstalls_only_installed_ports() {
        let macports = with_outdated("");

        let steps = macports
            .remove(&packages(&["git", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = macports
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["uninstall git"], commands(&steps));
        assert!(nothing.is_empty());
    }
}
