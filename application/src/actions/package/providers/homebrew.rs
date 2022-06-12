use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use std::{path::Path, process::Command};
use tracing::{debug, trace};
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Homebrew {}

impl PackageProvider for Homebrew {
    fn name(&self) -> &str {
        "Homebrew"
    }

    fn available(&self) -> bool {
        which("brew").is_ok()
    }

    fn bootstrap(&self) -> Vec<Step> {
        vec![Step { atom: Box::new(Exec {
            command: String::from("bash"),
            arguments: vec![
                String::from("-c"),
                String::from("$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)")
            ],
            ..Default::default()
        }), initializers: vec![], finalizers: vec![] },]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, repository: &PackageRepository) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("tap"), repository.name.clone()],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        let prefix = String::from_utf8(
            Command::new("brew")
                .arg("--prefix")
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .replace('\n', "")
        .replace('\r', "");

        let cellar = Path::new(&prefix).join("Cellar");
        let caskroom = Path::new(&prefix).join("Caskroom");

        package
            .packages()
            .into_iter()
            .filter(|p| {
                if cellar.join(&p).is_dir() {
                    trace!("{}: found in Cellar", p);
                    false
                } else if caskroom.join(&p).is_dir() {
                    trace!("{}: found in Caskroom", p);
                    false
                } else {
                    debug!("{}: doesn't appear to be installed", p);
                    true
                }
            })
            .collect()
    }

    fn install(&self, package: &PackageVariant) -> Vec<Step> {
        let need_installed = self.query(package);

        if need_installed.is_empty() {
            return vec![];
        }

        vec![Step {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: [
                    vec![String::from("install")],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
