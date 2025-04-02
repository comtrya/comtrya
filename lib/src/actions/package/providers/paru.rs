use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::once;
use std::process::Command;
use tracing::warn;
use tracing::{debug, trace};
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paru {}

impl PackageProvider for Paru {
    fn name(&self) -> &str {
        "Paru"
    }

    fn available(&self) -> bool {
        which("paru")
            .inspect_err(|_| warn!(message = "paru not available"))
            .is_ok()
    }

    fn bootstrap(&self, contexts: &Contexts) -> Vec<Step> {
        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or(String::from("sudo"));

        vec![
            Step {
                atom: Box::new(Exec {
                    command: String::from("pacman"),
                    arguments: vec![
                        String::from("-S"),
                        String::from("--noconfirm"),
                        String::from("--needed"),
                        String::from("base-devel"),
                        String::from("git"),
                    ],
                    privileged: true,
                    privilege_provider,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("git"),
                    arguments: vec![
                        String::from("clone"),
                        String::from("https://aur.archlinux.org/paru.git"),
                        String::from("/tmp/paru"),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Exec {
                    command: String::from("makepkg"),
                    arguments: vec![String::from("-si"), String::from("--noconfirm")],
                    working_dir: Some(String::from("/tmp/paru")),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
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
        let requested_already_installed: HashSet<String> = String::from_utf8(
            Command::new("pacman")
                .args(once(String::from("-Qq")).chain(package.packages()))
                .output()?
                .stdout,
        )?
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

    fn install(&self, package: &PackageVariant, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("paru"),
                arguments: [
                    vec![
                        String::from("-Sq"),
                        String::from("--sudoflags"),
                        String::from("-S"),
                        String::from("--batchinstall"),
                        String::from("--needed"),
                        String::from("--noconfirm"),
                        String::from("--noprogressbar"),
                        String::from("--skipreview"),
                        String::from("--useask"),
                    ],
                    package.extra_args.clone(),
                    package.packages(),
                ]
                .concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
