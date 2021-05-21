use super::PackageProvider;
use crate::{
    actions::{package::PackageVariant, ActionAtom},
    atoms::command::Exec,
};
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

    fn bootstrap(&self) -> Vec<ActionAtom> {
        vec![ActionAtom { atom: Box::new(Exec {
            command: String::from("bash"),
            arguments: vec![
                String::from("-c"),
                String::from("$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)")
            ],
            ..Default::default()
        }), initializers: vec![], finalizers: vec![] },]
    }

    fn has_repository(&self, _: &PackageVariant) -> bool {
        // Brew doesn't make it easy to check if the repository is already added
        // except by running `brew tap` and grepping.
        // Fortunately, adding an exist tap is pretty fast.
        false
    }

    fn add_repository(&self, package: &PackageVariant) -> Vec<ActionAtom> {
        let repository = package.repository.clone().unwrap();

        vec![ActionAtom {
            atom: Box::new(Exec {
                command: String::from("brew"),
                arguments: vec![String::from("tap"), repository],
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
        .replace("\n", "")
        .replace("\r", "");

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
            .map(|p| match &package.repository {
                Some(repository) => format!("{}/{}", repository, p),
                None => p,
            })
            .collect()
    }

    fn install(&self, package: &PackageVariant) -> Vec<ActionAtom> {
        let need_installed = self.query(package);

        if need_installed.is_empty() {
            return vec![];
        }

        vec![ActionAtom {
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
