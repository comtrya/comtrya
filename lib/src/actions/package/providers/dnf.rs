//! dnf on Fedora, Red Hat and CentOS

use super::PackageProvider;
use crate::actions::Result;

use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// dnf, as `dnf` or `yum`. Always runs `dnf install`, which skips installed packages
#[derive(Clone)]
pub struct Dnf {
    /// Runs `rpm -q -a` and `dnf list --upgrades` to find the installed and the upgradable
    /// packages
    runner: Arc<dyn Runner>,
}

impl Default for Dnf {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Dnf {
    /// The requested packages that rpm doesn't list as installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let command = CommandSpec::new("rpm").args(["-q", "-a", "--queryformat=%{NAME}\\n"]);
        let installed = super::listed(&*self.runner, &command, |line| Some(line))?;

        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// The requested packages that `dnf list --upgrades` lists, like
    /// `git.x86_64  2.47.1-1.fc41  updates`, by the name before the architecture
    fn outdated(&self, installed: &[String]) -> Result<Vec<String>> {
        if installed.is_empty() {
            return Ok(vec![]);
        }

        let command = CommandSpec::new("dnf").args(["list", "--upgrades", "--quiet"]);
        let upgradable = super::listed(&*self.runner, &command, |line| {
            let mut fields = line.split_whitespace();
            let (name, _version, _repository) = (fields.next()?, fields.next()?, fields.next()?);
            name.rsplit_once('.').map(|(name, _arch)| name)
        })?;

        Ok(installed
            .iter()
            .filter(|p| upgradable.contains(*p))
            .cloned()
            .collect())
    }

    /// `dnf <subcommand> --assumeyes --quiet`, with the privilege provider
    fn dnf(subcommand: &str, extra_args: &[String], packages: Vec<String>) -> Step {
        Step::new(Exec {
            command: String::from("dnf"),
            arguments: [
                vec![
                    String::from(subcommand),
                    String::from("--assumeyes"),
                    String::from("--quiet"),
                ],
                extra_args.to_vec(),
                packages,
            ]
            .concat(),
            ..Default::default()
        })
        .as_root()
    }
}

impl PackageProvider for Dnf {
    fn name(&self) -> &str {
        "DNF"
    }

    fn available(&self) -> bool {
        if which("dnf").is_ok() {
            true
        } else {
            warn!(message = "dnf not available");
            false
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("yum"),
            arguments: vec![
                String::from("install"),
                String::from("--assumeyes"),
                String::from("dnf"),
            ],
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, repository: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        if let Some(key) = repository.key.clone() {
            steps.extend(vec![Step::new(Exec {
                command: String::from("rpm"),
                arguments: vec![String::from("--import"), key.url],
                ..Default::default()
            })
            .as_root()]);
        }

        steps.extend(vec![
            Step::new(Exec {
                command: String::from("dnf"),
                arguments: vec![
                    String::from("config-manager"),
                    String::from("--assumeyes"),
                    String::from("--add-repo"),
                    repository.name.clone(),
                ],
                ..Default::default()
            })
            .as_root(),
            Step::new(Exec {
                command: String::from("dnf"),
                arguments: vec![
                    String::from("update"),
                    String::from("--assumeyes"),
                    String::from("--refresh"),
                ],
                ..Default::default()
            })
            .as_root(),
        ]);

        Ok(steps)
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Self::dnf(
            "install",
            &package.extra_args,
            self.query(package)?,
        )])
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let outdated = self.outdated(&super::installed(package, &missing))?;

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.push(Self::dnf("install", &package.extra_args, missing));
        }
        if !outdated.is_empty() {
            steps.push(Self::dnf("upgrade", &package.extra_args, outdated));
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
        Ok(vec![Self::dnf("remove", &[], installed)])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::{providers::PackageProviders, repository::RepositoryKey};
    use crate::contexts::Facts;

    use super::*;

    #[test]
    fn test_add_repository_without_key() {
        let dnf = Dnf::default();
        let facts = Facts::default();
        let steps = dnf.add_repository(
            &PackageRepository {
                name: String::from("test"),
                provider: Some(PackageProviders::Dnf),
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_repository_with_key() {
        let dnf = Dnf::default();
        let facts = Facts::default();
        let steps = dnf.add_repository(
            &PackageRepository {
                name: String::from("test"),
                key: Some(RepositoryKey {
                    url: String::from("abc"),
                    ..Default::default()
                }),
                provider: Some(PackageProviders::Dnf),
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 3);
    }

    /// dnf with `git` and `vim` installed, and `dnf list --upgrades` answering `upgradable`
    fn installed_git_and_vim(upgradable: &str) -> Dnf {
        use crate::runner::fake::{succeeds, FakeRunner};

        Dnf {
            runner: FakeRunner::new()
                .answer(
                    r"rpm -q -a --queryformat=%{NAME}\n",
                    succeeds("git\nvim-enhanced\nbash\n"),
                )
                .answer("dnf list --upgrades --quiet", succeeds(upgradable))
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
    fn latest_installs_missing_and_upgrades_outdated_packages() {
        let dnf = installed_git_and_vim(
            "Available Upgrades\nvim-enhanced.x86_64  2:9.1.866-1.fc41  updates\n",
        );

        let steps = dnf
            .upgrade(
                &packages(&["git", "vim-enhanced", "ripgrep"]),
                &Facts::default(),
            )
            .unwrap();

        assert_eq!(
            vec![
                "Run dnf install --assumeyes --quiet ripgrep (as root)",
                "Run dnf upgrade --assumeyes --quiet vim-enhanced (as root)",
            ],
            commands(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let dnf = installed_git_and_vim("");

        let steps = dnf.upgrade(&packages(&["git"]), &Facts::default()).unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_packages() {
        let dnf = installed_git_and_vim("");

        let steps = dnf
            .remove(&packages(&["git", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = dnf
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run dnf remove --assumeyes --quiet git (as root)"],
            commands(&steps)
        );
        assert!(nothing.is_empty());
    }
}
