use super::PackageProvider;
use crate::{
    actions::{package::PackageVariant, ActionAtom},
    atoms::command::Exec,
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Winget {}

impl PackageProvider for Winget {
    fn name(&self) -> &str {
        "Winget"
    }

    fn available(&self) -> bool {
        match which("winget") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "winget not available");
                false
            }
        }
    }

    fn bootstrap(&self) -> Vec<ActionAtom> {
        vec![]
    }

    fn has_repository(&self, _package: &PackageVariant) -> bool {
        true
    }

    fn add_repository(&self, _package: &PackageVariant) -> Vec<ActionAtom> {
        vec![]
    }

    fn query(&self, package: &PackageVariant) -> Vec<String> {
        // Install all packages, make this smarter soon
        package.packages()
    }

    fn install(&self, package: &PackageVariant) -> Vec<ActionAtom> {
        package
            .packages()
            .iter()
            .map::<ActionAtom, _>(|p| ActionAtom {
                atom: Box::new(Exec {
                    command: String::from("winget"),
                    arguments: [
                        vec![String::from("install"), String::from("--silent")],
                        package.extra_args.clone(),
                        vec![p.clone()],
                    ]
                    .concat(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect()
    }
}
