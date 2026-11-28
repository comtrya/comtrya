//! winget on Windows

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use std::sync::Arc;
use tracing::warn;
use which::which;

/// winget. Installs one package per step, from the `winget` source
#[derive(Clone)]
pub struct Winget {
    /// Runs `winget list` to find the installed packages
    runner: Arc<dyn Runner>,
}

impl Default for Winget {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl Winget {
    /// Whether `winget list` finds the package. A failing winget counts as not installed, so
    /// the install is tried anyway
    fn is_installed(&self, package: &str) -> bool {
        self.lists(package, list_args(package))
    }

    /// Whether `winget list --upgrade-available` finds the installed package. A failing
    /// winget counts as up to date
    fn is_outdated(&self, package: &str) -> bool {
        let mut args = list_args(package);
        args.push(String::from("--upgrade-available"));
        self.lists(package, args)
    }

    /// Whether `winget <args>` prints the package. winget exits with 0 even if nothing is
    /// found, and prints "No installed package found"
    fn lists(&self, package: &str, args: Vec<String>) -> bool {
        let output = match self.runner.run(&CommandSpec::new("winget").args(args)) {
            Ok(output) => output,
            Err(e) => {
                warn!("Failed to query winget for package {}: {}", package, e);
                return false;
            }
        };

        String::from_utf8_lossy(&output.stdout)
            .to_lowercase()
            .contains(&package.to_lowercase())
    }

    /// One step per package, as the user
    fn steps(packages: &[String], args: impl Fn(&str) -> Vec<String>) -> Vec<Step> {
        packages
            .iter()
            .map(|p| {
                Step::new(Exec {
                    command: String::from("winget"),
                    arguments: args(p),
                    ..Default::default()
                })
            })
            .collect()
    }
}

impl PackageProvider for Winget {
    fn name(&self) -> &str {
        "Winget"
    }

    fn available(&self) -> bool {
        if which("winget").is_ok() {
            true
        } else {
            warn!(message = "winget not available");
            false
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        true
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        Ok(super::missing(package, |p| self.is_installed(p)))
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        // does not require privilege escalation

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }
        // The executor runs the steps before it plans the next action, which asks again
        self.runner.forget();

        Ok(Self::steps(&need_installed, |p| {
            install_args(&package.extra_args, p)
        }))
    }

    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.query(package)?;
        let outdated: Vec<String> = super::installed(package, &missing)
            .into_iter()
            .filter(|p| self.is_outdated(p))
            .collect();
        if missing.is_empty() && outdated.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        let mut steps = Self::steps(&missing, |p| install_args(&package.extra_args, p));
        steps.extend(Self::steps(&outdated, |p| {
            upgrade_args(&package.extra_args, p)
        }));
        Ok(steps)
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.query(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(Self::steps(&installed, uninstall_args))
    }
}

/// `winget list -e --id <package>`. `--accept-source-agreements` keeps it from blocking on the
/// first run, `--disable-interactivity` stops it from prompting at all
fn list_args(package: &str) -> Vec<String> {
    [
        "list",
        "-e",
        "--id",
        package,
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
    .map(String::from)
    .to_vec()
}

/// `winget install` for one package, without any prompts
fn install_args(extra_args: &[String], package: &str) -> Vec<String> {
    changing_args("install", extra_args, package)
}

/// `winget upgrade` for one package, without any prompts
fn upgrade_args(extra_args: &[String], package: &str) -> Vec<String> {
    changing_args("upgrade", extra_args, package)
}

/// `winget uninstall -e --id <package>`, without any prompts
fn uninstall_args(package: &str) -> Vec<String> {
    [
        "uninstall",
        "-e",
        "--id",
        package,
        "--silent",
        "--accept-source-agreements",
        "--disable-interactivity",
    ]
    .map(String::from)
    .to_vec()
}

/// `winget <subcommand>` that installs a package from the `winget` source, without any prompts
fn changing_args(subcommand: &str, extra_args: &[String], package: &str) -> Vec<String> {
    [
        vec![
            subcommand.to_string(),
            "--silent".to_string(),
            "--accept-package-agreements".to_string(),
            "--accept-source-agreements".to_string(),
            "--disable-interactivity".to_string(),
            "--source".to_string(),
            "winget".to_string(),
        ],
        extra_args.to_vec(),
        vec![package.to_string()],
    ]
    .concat()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_reads_the_list_output_because_the_exit_code_is_always_0() {
        let list = |id: &str| format!("winget {}", list_args(id).join(" "));
        let winget = Winget {
            runner: FakeRunner::new()
                .answer(
                    &list("Git.Git"),
                    succeeds("Name Id      Version\nGit  Git.Git 2.47.0\n"),
                )
                .answer(
                    &list("BurntSushi.ripgrep.MSVC"),
                    succeeds("No installed package found matching input criteria.\n"),
                )
                .shared(),
        };
        let package = PackageVariant {
            list: vec![
                String::from("Git.Git"),
                String::from("BurntSushi.ripgrep.MSVC"),
                // winget fails to run: assumed missing, so it's installed
                String::from("Microsoft.PowerShell"),
            ],
            ..Default::default()
        };

        assert_eq!(
            vec!["BurntSushi.ripgrep.MSVC", "Microsoft.PowerShell"],
            winget.query(&package).unwrap()
        );
    }

    // winget must never wait for input, comtrya runs unattended
    const NON_INTERACTIVE: [&str; 2] = ["--accept-source-agreements", "--disable-interactivity"];

    #[test]
    fn list_is_non_interactive() {
        let args = list_args("Git.Git");

        assert_eq!(vec!["list", "-e", "--id", "Git.Git"], args[..4]);
        for flag in NON_INTERACTIVE {
            assert!(args.contains(&flag.to_string()), "missing {flag}");
        }
    }

    #[test]
    fn install_is_silent_and_non_interactive() {
        let args = install_args(&[], "Git.Git");

        assert_eq!("install", args[0]);
        for flag in NON_INTERACTIVE
            .iter()
            .chain(&["--silent", "--accept-package-agreements"])
        {
            assert!(args.contains(&flag.to_string()), "missing {flag}");
        }
    }

    #[test]
    fn latest_installs_missing_and_upgrades_outdated_packages() {
        let list = |id: &str| format!("winget {}", list_args(id).join(" "));
        let not_found = "No installed package found matching input criteria.\n";
        let winget = Winget {
            runner: FakeRunner::new()
                .answer(&list("Git.Git"), succeeds("Git  Git.Git 2.47.0\n"))
                .answer(
                    &format!("{} --upgrade-available", list("Git.Git")),
                    succeeds("Git  Git.Git 2.47.0 2.48.0 winget\n"),
                )
                .answer(
                    &list("Neovim.Neovim"),
                    succeeds("Neovim Neovim.Neovim 0.10\n"),
                )
                .answer(
                    &format!("{} --upgrade-available", list("Neovim.Neovim")),
                    succeeds(not_found),
                )
                .answer(&list("BurntSushi.ripgrep.MSVC"), succeeds(not_found))
                .shared(),
        };
        let package = PackageVariant {
            list: vec![
                String::from("Git.Git"),
                String::from("Neovim.Neovim"),
                String::from("BurntSushi.ripgrep.MSVC"),
            ],
            ..Default::default()
        };

        let steps: Vec<String> = winget
            .upgrade(&package, &Facts::default())
            .unwrap()
            .iter()
            .map(|step| step.atom.to_string())
            .collect();

        assert_eq!(2, steps.len());
        assert!(steps[0].starts_with("Run winget install"), "{}", steps[0]);
        assert!(
            steps[0].ends_with("BurntSushi.ripgrep.MSVC"),
            "{}",
            steps[0]
        );
        assert!(steps[1].starts_with("Run winget upgrade"), "{}", steps[1]);
        assert!(steps[1].ends_with("Git.Git"), "{}", steps[1]);
    }

    #[test]
    fn absent_uninstalls_only_installed_packages() {
        let list = |id: &str| format!("winget {}", list_args(id).join(" "));
        let winget = Winget {
            runner: FakeRunner::new()
                .answer(&list("Git.Git"), succeeds("Git  Git.Git 2.47.0\n"))
                .answer(
                    &list("Neovim.Neovim"),
                    succeeds("No installed package found\n"),
                )
                .shared(),
        };
        let package = PackageVariant {
            list: vec![String::from("Git.Git"), String::from("Neovim.Neovim")],
            ..Default::default()
        };

        let steps = winget.remove(&package, &Facts::default()).unwrap();

        assert_eq!(1, steps.len());
        assert_eq!(
            format!("Run winget {}", uninstall_args("Git.Git").join(" ")),
            steps[0].atom.to_string()
        );
    }

    #[test]
    fn upgrade_and_uninstall_are_non_interactive() {
        for args in [upgrade_args(&[], "Git.Git"), uninstall_args("Git.Git")] {
            for flag in NON_INTERACTIVE.iter().chain(&["--silent"]) {
                assert!(
                    args.contains(&flag.to_string()),
                    "missing {flag} in {args:?}"
                );
            }
        }
    }

    #[test]
    fn install_puts_extra_args_before_the_package() {
        let args = install_args(&["--scope".to_string(), "user".to_string()], "Git.Git");

        assert_eq!(vec!["--scope", "user", "Git.Git"], args[args.len() - 3..]);
    }
}
