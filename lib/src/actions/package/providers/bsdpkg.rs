//! pkg on FreeBSD and DragonFly. The only provider besides apt that installs local package files

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;

/// pkg. Always runs `pkg install`, which skips installed packages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BsdPkg {}

impl BsdPkg {
    /// Keeps pkg from asking questions
    fn env(&self) -> Vec<(String, String)> {
        vec![(String::from("ASSUME_ALWAYS_YES"), String::from("true"))]
    }
}

impl PackageProvider for BsdPkg {
    fn name(&self) -> &str {
        "BsdPkg"
    }

    fn available(&self) -> bool {
        match which("/usr/local/sbin/pkg") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "/usr/local/sbin/pkg is not available");
                false
            }
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self, facts: &Facts) -> Vec<Step> {
        vec![Step::new(Exec {
            command: String::from("/usr/sbin/pkg"),
            arguments: vec![String::from("bootstrap")],
            environment: self.env(),
            ..Default::default()
        })
        .as_root()]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        if package.file {
            return Ok(vec![Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("add")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                ..Default::default()
            })
            .as_root()]);
        }

        Ok(vec![
            // A dry run first: pkg may remove packages to install others
            Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("install"), String::from("-n")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                stop_if_stdout_contains: Some(String::from("removed")),
                ..Default::default()
            })
            .as_root(),
            Step::new(Exec {
                command: String::from("/usr/sbin/pkg"),
                arguments: vec![String::from("install"), String::from("-y")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                ..Default::default()
            })
            .as_root(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Atom;
    use pretty_assertions::assert_eq;

    /// The first step only looks: a dry run, which stops the action if pkg would remove
    /// packages. Only then pkg installs
    #[test]
    fn install_stops_when_a_dry_run_would_remove_packages() {
        let package = PackageVariant {
            list: vec![String::from("git")],
            ..Default::default()
        };

        let steps = BsdPkg {}.install(&package, &Facts::default()).unwrap();

        let commands: Vec<String> = steps.iter().map(Step::to_string).collect();
        assert_eq!(
            vec![
                "Run /usr/sbin/pkg install -n git (as root)",
                "Run /usr/sbin/pkg install -y git (as root)",
            ],
            commands
        );
        let Atom::Exec(dry_run) = &steps[0].atom else {
            panic!("the dry run is a program");
        };
        assert_eq!(
            Some(String::from("removed")),
            dry_run.stop_if_stdout_contains
        );
    }
}
