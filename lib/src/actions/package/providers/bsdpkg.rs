//! pkg on FreeBSD and DragonFly. The only provider besides apt that installs local package files

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::sync::Arc;
use tracing::{instrument, warn};
use which::which;

/// pkg. `state: present` always runs `pkg install`, which skips installed packages
#[derive(Clone)]
pub struct BsdPkg {
    /// Runs `pkg query %n` and `pkg version` to find the installed and the outdated packages
    runner: Arc<dyn Runner>,
}

impl Default for BsdPkg {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl BsdPkg {
    /// The requested packages that `pkg query %n` doesn't list as installed
    fn missing(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let command = CommandSpec::new("pkg").args(["query", "%n"]);
        let installed = super::listed(&*self.runner, &command, |line| Some(line))?;
        Ok(super::missing(package, |p| installed.contains(p)))
    }

    /// The packages of `installed` that the repository catalogue pkg has holds a newer
    /// version of (`-U`: without updating it). Lines look like `git-2.47.0   <`
    fn outdated(&self, installed: &[String]) -> Result<Vec<String>> {
        if installed.is_empty() {
            return Ok(vec![]);
        }

        let command = CommandSpec::new("pkg").args(["version", "-R", "-U", "-l", "<"]);
        let outdated = super::listed(&*self.runner, &command, |line| {
            let pkgver = line.split_whitespace().next()?;
            pkgver.rsplit_once('-').map(|(name, _version)| name)
        })?;
        Ok(super::among(installed, &outdated))
    }

    /// A dry run first, which stops when pkg would remove packages, then `pkg install`
    fn install_steps(extra_args: &[String], packages: &[String]) -> Vec<Step> {
        vec![
            // A dry run first: pkg may remove packages to install others
            Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: [&["install".into(), "-n".into()], extra_args, packages].concat(),
                stop_if_stdout_contains: Some(String::from("removed")),
                ..Default::default()
            })
            .as_root(),
            Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: [&["install".into(), "-y".into()], extra_args, packages].concat(),
                ..Default::default()
            })
            .as_root(),
        ]
    }

    /// `/usr/sbin/pkg <subcommand> -y <args> <packages>`, as root
    fn pkg(subcommand: &str, extra_args: &[String], packages: Vec<String>) -> Step {
        Step::new(Exec {
            command: String::from("/usr/sbin/pkg"),
            arguments: [
                vec![String::from(subcommand), String::from("-y")],
                extra_args.to_vec(),
                packages,
            ]
            .concat(),
            ..Default::default()
        })
        .as_root()
    }

    /// Keeps pkg from asking questions
    fn env(&self) -> Vec<(String, String)> {
        vec![(String::from("ASSUME_ALWAYS_YES"), String::from("true"))]
    }
}

impl PackageProvider for BsdPkg {
    fn name(&self) -> &str {
        "BsdPkg"
    }

    fn available(&self) -> bool {
        if which("/usr/local/sbin/pkg").is_ok() {
            true
        } else {
            warn!(message = "/usr/local/sbin/pkg is not available");
            false
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self, facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("/usr/sbin/pkg"),
            arguments: vec![String::from("bootstrap")],
            environment: self.env(),
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

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        if package.file {
            return Ok(vec![Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("add")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                ..Default::default()
            })
            .as_root()]);
        }

        Ok(Self::install_steps(
            &package.extra_args,
            &package.packages(),
        ))
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.missing(package)?;
        let outdated = self.outdated(&super::installed(package, &missing))?;

        let mut steps = vec![];
        if !missing.is_empty() {
            steps.extend(Self::install_steps(&package.extra_args, &missing));
        }
        if !outdated.is_empty() {
            steps.push(Self::pkg("upgrade", &package.extra_args, outdated));
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
        Ok(vec![Self::pkg("delete", &[], installed)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Atom;
    use pretty_assertions::assert_eq;

    /// The first step only looks: a dry run, which stops the action if pkg would remove
    /// packages. Only then pkg installs
    #[test]
    fn install_stops_when_a_dry_run_would_remove_packages() {
        let package = PackageVariant {
            list: vec![String::from("git")],
            ..Default::default()
        };

        let steps = BsdPkg::default()
            .install(&package, &Facts::default())
            .unwrap();

        let commands: Vec<String> = steps.iter().map(Step::to_string).collect();
        assert_eq!(
            vec![
                "Run /usr/sbin/pkg install -n git (as root)",
                "Run /usr/sbin/pkg install -y git (as root)",
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

    /// pkg with git and vim installed, and `outdated` from `pkg version`
    fn with_outdated(outdated: &str) -> BsdPkg {
        use crate::runner::fake::{succeeds, FakeRunner};

        BsdPkg {
            runner: FakeRunner::new()
                .answer("pkg query %n", succeeds("git\nvim\n"))
                .answer("pkg version -R -U -l <", succeeds(outdated))
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
        let steps = with_outdated("vim-9.1.0836                       <\n")
            .upgrade(&packages(&["git", "vim", "ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec![
                "Run /usr/sbin/pkg install -n ripgrep (as root)",
                "Run /usr/sbin/pkg install -y ripgrep (as root)",
                "Run /usr/sbin/pkg upgrade -y vim (as root)",
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
    fn absent_deletes_only_installed_packages() {
        let pkg = with_outdated("");

        let steps = pkg
            .remove(&packages(&["git", "ripgrep"]), &Facts::default())
            .unwrap();
        let nothing = pkg
            .remove(&packages(&["ripgrep"]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec!["Run /usr/sbin/pkg delete -y git (as root)"],
            commands(&steps)
        );
        assert!(nothing.is_empty());
    }
}
