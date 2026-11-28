//! Snaps on Linux. comtrya installs snapd with apt when it's missing

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::default::Default;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// snap, as `snapcraft` or `snap`. `state: present` always runs `snap install`
#[derive(Clone)]
pub struct Snapcraft {
    /// Runs `snap list` and `snap refresh --list` to find the installed and the outdated snaps
    runner: Arc<dyn Runner>,
}

impl Default for Snapcraft {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Snapcraft {
    /// The snaps `snap <args>` lists in its table, by the first column. The header row starts
    /// with `Name`; with nothing to list, snap prints a sentence to stderr instead
    fn listed(&self, args: &[&str]) -> Result<std::collections::HashSet<String>> {
        let command = CommandSpec::new("snap").args(args.iter().copied());
        super::listed(&*self.runner, &command, |line| {
            line.split_whitespace()
                .next()
                .filter(|name| *name != "Name")
        })
    }

    /// The requested snaps that aren't installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let installed = self.listed(&["list"])?;
        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// `snap <subcommand> <args> <snaps>`, as root
    fn snap(subcommand: &[&str], extra_args: &[String], snaps: Vec<String>) -> Step {
        let subcommand = subcommand.iter().map(ToString::to_string).collect();
        Step::new(Exec {
            command: String::from("snap"),
            arguments: [subcommand, extra_args.to_vec(), snaps].concat(),
            ..Default::default()
        })
        .as_root()
    }
}

impl PackageProvider for Snapcraft {
    fn name(&self) -> &str {
        "Snapcraft"
    }

    fn available(&self) -> bool {
        if which("snap").is_ok() {
            true
        } else {
            warn!(message = "snap is not available");
            false
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("apt"),
            arguments: vec![
                String::from("install"),
                String::from("--yes"),
                String::from("snapd"),
            ],
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _package: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _package: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("snap"),
            arguments: vec![String::from("install"), String::from("--yes")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            ..Default::default()
        })
        .as_root()])
    }

    /// Outdated snaps are what `snap refresh --list` names. snapd asks the store for it, there
    /// is no local list
    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let installed = super::installed(package, &missing);
        let outdated = if installed.is_empty() {
            vec![]
        } else {
            super::among(&installed, &self.listed(&["refresh", "--list"])?)
        };

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.push(Self::snap(
                &["install", "--yes"],
                &package.extra_args,
                missing,
            ));
        }
        if !outdated.is_empty() {
            steps.push(Self::snap(&["refresh"], &package.extra_args, outdated));
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
        Ok(vec![Self::snap(&["remove"], &[], installed)])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::providers::PackageProviders;
    use crate::contexts::Facts;

    use super::*;

    #[test]
    fn test_install() {
        let snapcraft = Snapcraft::default();
        let facts = Facts::default();
        let steps = snapcraft.install(
            &PackageVariant {
                name: Some(String::from("")),
                list: vec![],
                extra_args: vec![],
                provider: Some(PackageProviders::Snapcraft),
                file: false,
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }

    /// snap with core22 and firefox installed, and `refresh` from `snap refresh --list`
    fn with_refreshes(refresh: &str) -> Snapcraft {
        use crate::runner::fake::{succeeds, FakeRunner};

        Snapcraft {
            runner: FakeRunner::new()
                .answer(
                    "snap list",
                    succeeds(
                        "Name     Version  Rev   Tracking       Publisher  Notes\n\
                         core22   20241001 1663  latest/stable  canonical  base\n\
                         firefox  132.0    5187  latest/stable  mozilla    -\n",
                    ),
                )
                .answer("snap refresh --list", succeeds(refresh))
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
    fn latest_installs_missing_and_refreshes_outdated_snaps() {
        let steps = with_refreshes(
            "Name     Version  Rev   Size   Publisher  Notes\nfirefox  133.0    5273  280MB  mozilla    -\n",
        )
        .upgrade(&packages(&["core22", "firefox", "htop"]), &Facts::default())
        .unwrap();

        assert_eq!(
            vec![
                "Run snap install --yes htop (as root)",
                "Run snap refresh firefox (as root)",
            ],
            commands(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let steps = with_refreshes("")
            .upgrade(&packages(&["firefox"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_snaps() {
        let snap = with_refreshes("");

        let steps = snap
            .remove(&packages(&["firefox", "htop"]), &Facts::default())
            .unwrap();
        let nothing = snap
            .remove(&packages(&["htop"]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["Run snap remove firefox (as root)"], commands(&steps));
        assert!(nothing.is_empty());
    }
}
