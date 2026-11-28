//! zypper on openSUSE and SUSE

use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::actions::Result;
use crate::atoms::command::Exec;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use std::sync::Arc;
use tracing::warn;
use which::which;

/// zypper. `state: present` always runs `zypper install`, which skips installed packages
#[derive(Clone)]
pub struct Zypper {
    /// Runs `rpm -q -a` and `zypper list-updates` to find the installed and the upgradable
    /// packages
    runner: Arc<dyn Runner>,
}

impl Default for Zypper {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Zypper {
    /// The requested packages that rpm doesn't list as installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let command = CommandSpec::new("rpm").args(["-q", "-a", "--queryformat=%{NAME}\\n"]);
        let installed = super::listed(&*self.runner, &command, |line| Some(line))?;

        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// The packages of `installed` that `zypper list-updates` lists, from the repository data
    /// zypper has (`--no-refresh`). Its table rows look like
    /// `v | repo-oss | git | 2.47.0-1.1 | 2.47.1-1.1 | x86_64`, the name is the third column
    fn outdated(&self, installed: &[String]) -> Result<Vec<String>> {
        if installed.is_empty() {
            return Ok(vec![]);
        }

        let command =
            CommandSpec::new("zypper").args(["--non-interactive", "--no-refresh", "list-updates"]);
        let upgradable = super::listed(&*self.runner, &command, |line| {
            let mut columns = line.split('|').map(str::trim);
            (columns.next()? == "v").then_some(())?;
            columns.nth(1).filter(|name| !name.is_empty())
        })?;

        Ok(super::among(installed, &upgradable))
    }

    /// `zypper --non-interactive <subcommand> <args> <packages>`, as root
    fn zypper(subcommand: &str, extra_args: &[String], packages: Vec<String>) -> Step {
        Step::new(Exec {
            command: String::from("zypper"),
            arguments: [
                vec![String::from("--non-interactive"), String::from(subcommand)],
                extra_args.to_vec(),
                packages,
            ]
            .concat(),
            ..Default::default()
        })
        .as_root()
    }
}

impl PackageProvider for Zypper {
    fn name(&self) -> &str {
        "Zypper"
    }

    fn available(&self) -> bool {
        if which("zypper").is_ok() {
            true
        } else {
            warn!(message = "zypper not available");
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
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("zypper"),
            arguments: vec![String::from("install"), String::from("-y")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            ..Default::default()
        })
        .as_root()])
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let outdated = self.outdated(&super::installed(package, &missing))?;

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.push(Self::zypper("install", &package.extra_args, missing));
        }
        if !outdated.is_empty() {
            steps.push(Self::zypper("update", &package.extra_args, outdated));
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
        Ok(vec![Self::zypper("remove", &[], installed)])
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::package::providers::PackageProviders;
    use crate::runner::fake::{succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_install() {
        let zypper = Zypper::default();
        let facts = Facts::default();
        let steps = zypper.install(
            &PackageVariant {
                name: Some(String::from("")),
                list: vec![],
                extra_args: vec![],
                provider: Some(PackageProviders::Zypper),
                file: false,
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }

    /// From `zypper list-updates` on openSUSE Tumbleweed, shortened
    const UPDATES: &str = "\
S | Repository | Name | Current Version | Available Version | Arch
--+------------+------+-----------------+-------------------+-------
v | repo-oss   | vim  | 9.1.0836-1.1    | 9.1.0866-1.1      | x86_64
";

    /// zypper with `git` and `vim` installed, and `updates` from `zypper list-updates`
    fn installed_git_and_vim(updates: &str) -> Zypper {
        Zypper {
            runner: FakeRunner::new()
                .answer(
                    r"rpm -q -a --queryformat=%{NAME}\n",
                    succeeds("git\nvim\nbash\n"),
                )
                .answer(
                    "zypper --non-interactive --no-refresh list-updates",
                    succeeds(updates),
                )
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
        let zypper = installed_git_and_vim(UPDATES);

        let steps = zypper
            .upgrade(&packages(&["git", "vim", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec![
                "Run zypper --non-interactive install ripgrep (as root)",
                "Run zypper --non-interactive update vim (as root)",
            ],
            commands(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let zypper = installed_git_and_vim("No updates found.\n");

        let steps = zypper
            .upgrade(&packages(&["git", "vim"]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_only_installed_packages() {
        let zypper = installed_git_and_vim("");

        let steps = zypper
            .remove(&packages(&["git", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = zypper
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run zypper --non-interactive remove git (as root)"],
            commands(&steps)
        );
        assert!(nothing.is_empty());
    }
}
