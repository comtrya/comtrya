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
    /// Runs `luarocks list --porcelain` to find the installed rocks
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
        match which("luarocks") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "luarocks not available");
                false
            }
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
            list: names.iter().map(|name| name.to_string()).collect(),
            extra_args: extra_args.iter().map(|arg| arg.to_string()).collect(),
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
            vec![
                "CommandExec with: luarocks install lgi",
                "CommandExec with: luarocks install path"
            ],
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
            "CommandExec with: luarocks install --local lgi",
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
            format!("{:#}", anyhow::Error::from(error))
        );
    }
}
