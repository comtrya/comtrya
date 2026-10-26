use super::PackageProvider;
use crate::actions::package::{repository::PackageRepository, PackageVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;
use tracing::{debug, trace, warn};
use which::which;

/// Packages from the official Arch repositories. AUR packages need `yay` or `paru`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pacman {}

impl PackageProvider for Pacman {
    fn name(&self) -> &str {
        "Pacman"
    }

    fn available(&self) -> bool {
        match which("pacman") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "pacman not available");
                false
            }
        }
    }

    fn bootstrap(&self, _contexts: &Contexts) -> Vec<Step> {
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(
        &self,
        _repository: &PackageRepository,
        _contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        // `pacman -Qq` prints the installed ones and fails for the others, so ignore the status
        let installed: HashSet<String> = String::from_utf8(
            Command::new("pacman")
                .args(["-Q", "-q"])
                .args(package.packages())
                .output()?
                .stdout,
        )?
        .lines()
        .map(String::from)
        .collect();

        Ok(package
            .packages()
            .into_iter()
            .filter(|p| {
                if installed.contains(p) {
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
        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        Ok(vec![install_step(package, need_installed, contexts)])
    }
}

fn install_step(package: &PackageVariant, packages: Vec<String>, contexts: &Contexts) -> Step {
    let privilege_provider = utilities::get_privilege_provider(contexts);

    Step {
        atom: Box::new(Exec {
            command: String::from("pacman"),
            arguments: [
                vec![
                    String::from("-S"),
                    String::from("--needed"),
                    String::from("--noconfirm"),
                ],
                package.extra_args.clone(),
                packages,
            ]
            .concat(),
            privileged: true,
            privilege_provider,
            ..Default::default()
        }),
        initializers: vec![],
        finalizers: vec![],
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::actions::package::providers::PackageProviders;
    use pretty_assertions::assert_eq;

    #[test]
    fn installs_with_privileges() {
        let package = PackageVariant {
            name: None,
            list: vec![String::from("git"), String::from("ripgrep")],
            extra_args: vec![String::from("--asdeps")],
            provider: Some(PackageProviders::Pacman),
            file: false,
        };

        let step = install_step(&package, package.packages(), &Contexts::default());

        assert_eq!(
            "CommandExec with: privileged=true: pacman -S --needed --noconfirm --asdeps git ripgrep",
            step.atom.to_string()
        );
    }
}
