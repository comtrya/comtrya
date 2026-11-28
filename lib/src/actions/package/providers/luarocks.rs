//! LuaRocks, the package manager for Lua modules

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

/// LuaRocks. Rocks go into the system tree as root, or into the user's tree with `--local` in
/// `extra_args`, which needs no root
#[derive(Clone)]
pub struct Luarocks {
    /// Runs `luarocks list --porcelain` to find the installed rocks, with `--outdated` the
    /// outdated ones
    runner: Arc<dyn Runner>,
}

impl Default for Luarocks {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Luarocks {
    fn name(&self) -> &str {
        "Luarocks"
    }

    fn available(&self) -> bool {
        if which("luarocks").is_ok() {
            true
        } else {
            warn!(message = "luarocks not available");
            false
        }
    }

    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        warn!(message = "luarocks has no repositories to add, set a server with extra_args");
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        let command = CommandSpec::new("luarocks").args(["list", "--porcelain"]);
        let output = super::query(&*self.runner, &command)?;
        if !output.success {
            return Err(Error::Query {
                command: command.to_string(),
                source: String::from_utf8_lossy(&output.stderr).trim().into(),
            });
        }
        let list = String::from_utf8_lossy(&output.stdout);
        let installed = installed_rocks(&list);

        Ok(super::missing(package, |rock| installed.contains(rock)))
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }
        // The executor runs the steps before it plans the next action, which reads the list again
        self.runner.forget();

        // `luarocks install` takes one rock at a time
        Ok(need_installed
            .into_iter()
            .map(|rock| install_step(package, rock))
            .collect())
    }

    /// Outdated rocks are what `luarocks list --outdated` names: a newer version on the rocks
    /// server, which luarocks asks
    fn upgrade(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let missing = self.query(package)?;
        let installed = super::installed(package, &missing);
        let outdated = if installed.is_empty() {
            vec![]
        } else {
            let command = CommandSpec::new("luarocks").args(["list", "--outdated", "--porcelain"]);
            let list = String::from_utf8_lossy(&super::query(&*self.runner, &command)?.stdout)
                .into_owned();
            let outdated: HashSet<String> = installed_rocks(&list)
                .into_iter()
                .map(String::from)
                .collect();
            super::among(&installed, &outdated)
        };
        let targets: Vec<String> = package
            .packages()
            .into_iter()
            .filter(|rock| missing.contains(rock) || outdated.contains(rock))
            .collect();
        if targets.is_empty() {
            return Ok(vec![]);
        }

        // `luarocks install` installs the newest version, also over an older one
        self.runner.forget();
        Ok(targets
            .into_iter()
            .map(|rock| install_step(package, rock))
            .collect())
    }

    fn remove(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        let installed = super::installed(package, &self.query(package)?);
        if installed.is_empty() {
            return Ok(vec![]);
        }

        self.runner.forget();
        Ok(installed
            .into_iter()
            .map(|rock| remove_step(package, rock))
            .collect())
    }
}

/// `luarocks remove <rock>` in the tree the rocks went into: only the tree arguments of
/// `extra_args` are passed, install arguments like `--server` aren't known to `remove`
fn remove_step(package: &PackageVariant, rock: String) -> Step {
    let step = Step::new(Exec {
        command: String::from("luarocks"),
        arguments: [
            vec![String::from("remove")],
            tree_args(&package.extra_args),
            vec![rock],
        ]
        .concat(),
        ..Default::default()
    });

    if user_tree(&package.extra_args) {
        step
    } else {
        step.as_root()
    }
}

/// The arguments of `extra_args` that pick a tree: `--local`, `--tree=<dir>`, `--tree <dir>`
fn tree_args(extra_args: &[String]) -> Vec<String> {
    let mut args = vec![];
    let mut next_is_tree = false;
    for arg in extra_args {
        if next_is_tree || arg == "--local" || arg.starts_with("--tree=") {
            args.push(arg.clone());
            next_is_tree = false;
        } else if arg == "--tree" {
            args.push(arg.clone());
            next_is_tree = true;
        }
    }
    args
}

/// `luarocks install <rock>`, as root unless it goes into the user's tree
fn install_step(package: &PackageVariant, rock: String) -> Step {
    let step = Step::new(Exec {
        command: String::from("luarocks"),
        arguments: [
            vec![String::from("install")],
            package.extra_args.clone(),
            vec![rock],
        ]
        .concat(),
        ..Default::default()
    });

    if user_tree(&package.extra_args) {
        step
    } else {
        step.as_root()
    }
}

/// Whether `extra_args` install into the user's tree (`--local`) or a tree of their own
/// (`--tree`), which don't need root
fn user_tree(extra_args: &[String]) -> bool {
    extra_args
        .iter()
        .any(|arg| arg == "--local" || arg == "--tree" || arg.starts_with("--tree="))
}

/// The names of the installed rocks in `luarocks list --porcelain`, whose lines are
/// `<name>\t<version>\tinstalled\t<tree>`
fn installed_rocks(list: &str) -> HashSet<&str> {
    list.lines()
        .filter_map(|line| line.split('\t').next())
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::RunAs;
    use crate::runner::fake::{fails, succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    /// From `luarocks list --porcelain` 3.11 with a system and a user tree
    const LIST: &str = "\
lpeg\t1.1.0-2\tinstalled\t/usr/local/lib/luarocks/rocks-5.4
luafilesystem\t1.8.0-1\tinstalled\t/usr/local/lib/luarocks/rocks-5.4
lyaml\t6.2.8-1\tinstalled\t/home/ada/.luarocks/lib/luarocks/rocks-5.4
";

    fn luarocks(runner: &Arc<FakeRunner>) -> Luarocks {
        Luarocks {
            runner: runner.clone(),
        }
    }

    fn rocks(names: &[&str], extra_args: &[&str]) -> PackageVariant {
        PackageVariant {
            list: names.iter().map(std::string::ToString::to_string).collect(),
            extra_args: extra_args
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn installed_rocks_are_read_from_the_list() {
        assert_eq!(
            HashSet::from(["lpeg", "luafilesystem", "lyaml"]),
            installed_rocks(LIST)
        );
    }

    #[test]
    fn query_returns_the_rocks_that_are_not_installed() {
        let runner = FakeRunner::new()
            .answer("luarocks list --porcelain", succeeds(LIST))
            .shared();

        assert_eq!(
            vec!["lgi", "lua"],
            luarocks(&runner)
                .query(&rocks(&["lpeg", "lgi", "lyaml", "lua"], &[]))
                .unwrap()
        );
        assert_eq!(vec!["luarocks list --porcelain"], runner.calls());
    }

    #[test]
    fn each_missing_rock_is_installed_as_root() {
        let runner = FakeRunner::new()
            .answer("luarocks list --porcelain", succeeds(LIST))
            .shared();

        let steps = luarocks(&runner)
            .install(&rocks(&["lpeg", "lgi", "path"], &[]), &Facts::default())
            .unwrap();

        let shown: Vec<String> = steps.iter().map(|step| step.atom.to_string()).collect();
        assert_eq!(
            vec!["Run luarocks install lgi", "Run luarocks install path"],
            shown
        );
        assert!(steps.iter().all(|step| step.run_as == RunAs::Root));
    }

    #[test]
    fn rocks_for_the_users_tree_need_no_root() {
        let runner = FakeRunner::new()
            .answer("luarocks list --porcelain", succeeds(LIST))
            .shared();

        let steps = luarocks(&runner)
            .install(&rocks(&["lgi"], &["--local"]), &Facts::default())
            .unwrap();

        assert_eq!(
            "Run luarocks install --local lgi",
            steps[0].atom.to_string()
        );
        assert_eq!(RunAs::User, steps[0].run_as);
        assert!(user_tree(&[String::from("--tree=/opt/rocks")]));
    }

    #[test]
    fn nothing_to_install_plans_nothing() {
        let runner = FakeRunner::new()
            .answer("luarocks list --porcelain", succeeds(LIST))
            .shared();

        assert!(luarocks(&runner)
            .install(&rocks(&["lpeg"], &[]), &Facts::default())
            .unwrap()
            .is_empty());
    }

    /// `luarocks list --outdated --porcelain` names lpeg
    fn with_outdated_lpeg() -> Arc<FakeRunner> {
        FakeRunner::new()
            .answer("luarocks list --porcelain", succeeds(LIST))
            .answer(
                "luarocks list --outdated --porcelain",
                succeeds("lpeg\t1.1.0-2\t1.1.0-3\thttps://luarocks.org\n"),
            )
            .shared()
    }

    fn shown(steps: &[Step]) -> Vec<String> {
        steps.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn latest_installs_missing_and_outdated_rocks() {
        let steps = luarocks(&with_outdated_lpeg())
            .upgrade(&rocks(&["lpeg", "lyaml", "lgi"], &[]), &Facts::default())
            .unwrap();

        assert_eq!(
            vec![
                "Run luarocks install lpeg (as root)",
                "Run luarocks install lgi (as root)",
            ],
            shown(&steps)
        );
    }

    #[test]
    fn latest_does_nothing_when_everything_is_up_to_date() {
        let steps = luarocks(&with_outdated_lpeg())
            .upgrade(&rocks(&["lyaml"], &[]), &Facts::default())
            .unwrap();

        assert!(steps.is_empty());
    }

    #[test]
    fn absent_removes_installed_rocks_from_their_tree() {
        let runner = with_outdated_lpeg();

        let steps = luarocks(&runner)
            .remove(
                &rocks(
                    &["lyaml", "lgi"],
                    &["--server=https://example.com", "--local"],
                ),
                &Facts::default(),
            )
            .unwrap();
        let nothing = luarocks(&runner)
            .remove(&rocks(&["lgi"], &[]), &Facts::default())
            .unwrap();

        assert_eq!(vec!["Run luarocks remove --local lyaml"], shown(&steps));
        assert!(nothing.is_empty());
        assert_eq!(
            vec!["--tree", "/opt/rocks"],
            tree_args(&["--tree".into(), "/opt/rocks".into(), "--server=x".into()])
        );
    }

    #[test]
    fn a_failing_query_is_an_error() {
        let runner = FakeRunner::new()
            .answer(
                "luarocks list --porcelain",
                fails("", "Error: no rocks trees"),
            )
            .shared();

        let error = luarocks(&runner).query(&rocks(&["lpeg"], &[])).unwrap_err();

        assert_eq!(
            "can't list the installed packages with `luarocks list --porcelain`: Error: no rocks trees",
            crate::utilities::error_chain(&error)
        );
    }
}
