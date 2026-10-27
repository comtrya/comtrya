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
pub struct Yay {
    runner: Arc<dyn Runner>,
}

impl Default for Yay {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Yay {
    fn name(&self) -> &str {
        "Yay"
    }

    fn available(&self) -> bool {
        match which("yay") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "yay not available");
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
                    String::from("--needed"),
                    String::from("--noconfirm"),
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
                    String::from("https://aur.archlinux.org/yay.git"),
                    String::from("/tmp/yay"),
                ],
                ..Default::default()
            }),
            Step::new(Exec {
                command: String::from("makepkg"),
                arguments: vec![String::from("-si"), String::from("--noconfirm")],
                working_dir: Some(String::from("/tmp/yay")),
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
            .args(["-Q", "-q"])
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
        // yay runs as the user and calls sudo itself
        super::refuse_root("yay", contexts)?;

        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![Step::new(Exec {
            command: String::from("yay"),
            arguments: [
                vec![String::from("-S"), String::from("--noconfirm")],
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
    use crate::runner::fake::{fails, FakeRunner};
    use pretty_assertions::assert_eq;

    #[test]
    fn query_asks_pacman_what_is_installed() {
        let runner = FakeRunner::new()
            .answer(
                "pacman -Q -q git yay-bin",
                fails("git\n", "error: package 'yay-bin' was not found\n"),
            )
            .shared();
        let yay = Yay {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            list: vec![String::from("git"), String::from("yay-bin")],
            ..Default::default()
        };

        assert_eq!(vec!["yay-bin"], yay.query(&package).unwrap());
        assert_eq!(vec!["pacman -Q -q git yay-bin"], runner.calls());
    }
}
