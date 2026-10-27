use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::utilities;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::warn;
use tracing::{debug, trace};
use which::which;

#[derive(Clone)]
pub struct Paru {
    runner: Arc<dyn Runner>,
}

impl Default for Paru {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Paru {
    fn name(&self) -> &str {
        "Paru"
    }

    fn available(&self) -> bool {
        match which("paru") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "paru not available");
                false
            }
        }
    }

    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        let privilege_provider = utilities::get_privilege_provider(contexts);

        vec![
            Step::new(Exec {
                command: String::from("pacman"),
                arguments: vec![
                    String::from("-S"),
                    String::from("--noconfirm"),
                    String::from("--needed"),
                    String::from("base-devel"),
                    String::from("git"),
                ],
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            Step::new(Exec {
                command: String::from("git"),
                arguments: vec![
                    String::from("clone"),
                    String::from("https://aur.archlinux.org/paru.git"),
                    String::from("/tmp/paru"),
                ],
                ..Default::default()
            }),
            Step::new(Exec {
                command: String::from("makepkg"),
                arguments: vec![String::from("-si"), String::from("--noconfirm")],
                working_dir: Some(String::from("/tmp/paru")),
                ..Default::default()
            }),
        ]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        _: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        let command = CommandSpec::new("pacman")
            .args(["-Qq"])
            .args(package.packages());
        let requested_already_installed: HashSet<String> =
            String::from_utf8(self.runner.run(&command)?.stdout)?
                .split('\n')
                .map(String::from)
                .collect();

        debug!(
            "all requested installed packages: {:?}",
            requested_already_installed
        );

        Ok(package
            .packages()
            .into_iter()
            .filter(|p| {
                if requested_already_installed.contains(p) {
                    trace!("{}: already installed", p);
                    false
                } else {
                    debug!("{}: doesn't appear to be installed", p);
                    true
                }
            })
            .collect())
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        // paru runs as the user and calls sudo itself
        super::refuse_root("paru", contexts)?;

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step::new(Exec {
            command: String::from("paru"),
            arguments: [
                vec![
                    String::from("-Sq"),
                    String::from("--batchinstall"),
                    String::from("--needed"),
                    String::from("--noconfirm"),
                    String::from("--noprogressbar"),
                    String::from("--skipreview"),
                    String::from("--sudoloop"),
                    String::from("--useask"),
                ],
                package.extra_args.clone(),
                need_installed,
            ]
            .concat(),
            ..Default::default()
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fails, succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_asks_pacman_what_is_installed() {
        let runner = FakeRunner::new()
            .answer(
                "pacman -Qq git yay-bin",
                fails("git\n", "error: package 'yay-bin' was not found\n"),
            )
            .shared();
        let paru = Paru {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            list: vec![String::from("git"), String::from("yay-bin")],
            ..Default::default()
        };

        assert_eq!(vec!["yay-bin"], paru.query(&package).unwrap());
        assert_eq!(vec!["pacman -Qq git yay-bin"], runner.calls());
    }

    fn paru(runner: FakeRunner) -> Paru {
        Paru {
            runner: runner.shared(),
        }
    }

    fn package(list: &[&str]) -> PackageVariant {
        PackageVariant {
            list: list.iter().map(|p| p.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn install_skips_installed_packages() {
        let runner = FakeRunner::new().answer(
            "pacman -Qq git yay-bin",
            fails("git\n", "error: package 'yay-bin' was not found\n"),
        );

        let steps = paru(runner)
            .install(&package(&["git", "yay-bin"]), &Contexts::default())
            .unwrap();

        assert_eq!(1, steps.len());
        assert!(
            steps[0].atom.to_string().ends_with("--useask yay-bin"),
            "{}",
            steps[0].atom
        );
    }

    #[test]
    fn install_does_nothing_when_everything_is_installed() {
        let runner = FakeRunner::new().answer("pacman -Qq git", succeeds("git\n"));

        let steps = paru(runner)
            .install(&package(&["git"]), &Contexts::default())
            .unwrap();

        assert!(steps.is_empty());
    }
}
