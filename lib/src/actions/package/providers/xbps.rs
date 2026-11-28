//! xbps on Void Linux

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::package::PackageVariant;
use crate::actions::{Error, Result};
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// xbps
#[derive(Clone)]
pub struct Xbps {
    /// Runs `xbps-query -l` and `xbps-install -n -u` to find the installed and the upgradable
    /// packages
    runner: Arc<dyn Runner>,
}

impl Xbps {
    /// The packages of `installed` that a dry run of a system update would update, from the
    /// repository data xbps has (no `-S`)
    fn outdated(&self, installed: &[String]) -> Result<Vec<String>> {
        if installed.is_empty() {
            return Ok(vec![]);
        }

        let command = CommandSpec::new("xbps-install").args(["-n", "-u"]);
        let updates = super::listed(&*self.runner, &command, update_name)?;

        Ok(super::among(installed, &updates))
    }

    /// `xbps-install -S --yes --update <packages>`, which installs missing and updates
    /// installed packages, as root
    fn install_step(package: &PackageVariant, packages: Vec<String>) -> Step {
        Step::new(Exec {
            command: String::from("xbps-install"),
            arguments: [
                vec![
                    String::from("-S"),
                    String::from("--yes"),
                    String::from("--update"),
                ],
                package.extra_args.clone(),
                packages,
            ]
            .concat(),
            ..Default::default()
        })
        .as_root()
    }
}

impl Default for Xbps {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Xbps {
    fn name(&self) -> &str {
        "Xbps"
    }

    fn available(&self) -> bool {
        let mut install = false;
        let mut query = false;
        match which("xbps-install") {
            Ok(_) => {
                install = true;
            }
            Err(_) => {
                warn!(message = "xbps-install not available");
            }
        };
        match which("xbps-query") {
            Ok(_) => {
                query = true;
            }
            Err(_) => {
                warn!(message = "xbps-query not available");
            }
        };
        install && query
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let command = CommandSpec::new("xbps-query").arg("-l");
        let output = super::query(&*self.runner, &command)?;
        if !output.success {
            return Err(Error::Query {
                command: command.to_string(),
                source: String::from_utf8_lossy(&output.stderr).trim().into(),
            });
        }
        let list = String::from_utf8_lossy(&output.stdout);
        let installed = installed_packages(&list);

        Ok(super::missing(package, |p| installed.contains(p)))
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }
        // The executor runs the step before it plans the next action, which reads the list again
        self.runner.forget();

        Ok(vec![Self::install_step(package, need_installed)])
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.query(package)?;
        let outdated = self.outdated(&super::installed(package, &missing))?;
        let targets: Vec<String> = package
            .packages()
            .into_iter()
            .filter(|p| missing.contains(p) || outdated.contains(p))
            .collect();
        if targets.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![Self::install_step(package, targets)])
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.query(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(vec![Step::new(Exec {
            command: String::from("xbps-remove"),
            arguments: [vec![String::from("--yes")], installed].concat(),
            ..Default::default()
        })
        .as_root()])
    }
}

/// The name of the package on a line of `xbps-install -n -u`, like
/// `vim-9.1.0866_1 update x86_64 https://repo-default.voidlinux.org/current 12345 6789`, if
/// the line is an update
fn update_name(line: &str) -> Option<&str> {
    let mut fields = line.split_whitespace();
    let pkgver = fields.next()?;
    (fields.next()? == "update").then_some(())?;
    pkgver.rsplit_once('-').map(|(name, _version)| name)
}

/// The names of the installed packages in `xbps-query -l`, whose lines look like
/// `ii git-2.43.0_1  Git Tree History Storage Tool`. `ii` means installed, and the name is the
/// part of `<name>-<version>_<revision>` before the last `-`
fn installed_packages(list: &str) -> HashSet<&str> {
    list.lines()
        .filter_map(|line| line.strip_prefix("ii "))
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|pkgver| pkgver.rsplit_once('-'))
        .map(|(name, _version)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fails, succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    /// Shortened from Void Linux
    const LIST: &str = "\
ii bash-5.2.037_1                     GNU Bourne Again Shell
ii ca-certificates-20250419+3.131.1_2 Common CA certificates for SSL/TLS from Mozilla
ii python3-yaml-6.0.2_2               YAML parser and emitter for Python3
uu half-installed-1.0_1               Unpacked, but not configured
";

    #[test]
    fn installed_packages_are_read_from_the_list() {
        assert_eq!(
            HashSet::from(["bash", "ca-certificates", "python3-yaml"]),
            installed_packages(LIST)
        );
    }

    #[test]
    fn query_returns_the_packages_that_are_not_installed() {
        let runner = FakeRunner::new()
            .answer("xbps-query -l", succeeds(LIST))
            .shared();
        let xbps = Xbps {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            // "dash" matches the description of bash, "python3" is the start of python3-yaml
            list: ["bash", "python3-yaml", "dash", "python3", "half-installed"]
                .map(String::from)
                .to_vec(),
            ..Default::default()
        };

        assert_eq!(
            vec!["dash", "python3", "half-installed"],
            xbps.query(&package).unwrap()
        );
        assert_eq!(vec!["xbps-query -l"], runner.calls());
    }

    /// xbps with LIST installed and `updates` from a dry run of a system update
    fn with_updates(updates: &str) -> Xbps {
        Xbps {
            runner: FakeRunner::new()
                .answer("xbps-query -l", succeeds(LIST))
                .answer("xbps-install -n -u", succeeds(updates))
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
    fn latest_installs_missing_and_updates_outdated_packages() {
        let xbps = with_updates(
            "bash-5.2.037_2 update x86_64 https://repo-default.voidlinux.org/current 1 2\n\
             zsh-5.9_6 install x86_64 https://repo-default.voidlinux.org/current 1 2\n",
        );

        let steps = xbps
            .upgrade(
                &packages(&["bash", "python3-yaml", "zsh"]),
                &Facts::default(),
            )
            .unwrap();

        assert_eq!(
            vec!["Run xbps-install -S --yes --update bash zsh (as root)"],
            commands(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let steps = with_updates("")
            .upgrade(&packages(&["bash"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_packages() {
        let xbps = with_updates("");

        let steps = xbps
            .remove(&packages(&["bash", "zsh"]), &Facts::default())
            .unwrap();
        let nothing = xbps.remove(&packages(&["zsh"]), &Facts::default()).unwrap();

        assert_eq!(
            vec!["Run xbps-remove --yes bash (as root)"],
            commands(&steps)
        );
        assert!(nothing.is_empty());
    }

    #[test]
    fn a_failing_query_is_an_error() {
        let xbps = Xbps {
            runner: FakeRunner::new()
                .answer("xbps-query -l", fails("", "ERROR: can't open pkgdb"))
                .shared(),
        };
        let package = PackageVariant {
            name: Some(String::from("bash")),
            ..Default::default()
        };

        let error = crate::utilities::error_chain(&xbps.query(&package).unwrap_err());
        assert_eq!(
            "can't list the installed packages with `xbps-query -l`: ERROR: can't open pkgdb",
            error
        );
    }
}
