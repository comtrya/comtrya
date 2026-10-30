//! pkgin on NetBSD, and on other systems with pkgsrc

use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::Result;
use crate::contexts::Facts;
use crate::steps::finalizers::FlowControl::StopIf;
use crate::steps::finalizers::OutputContains;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;

/// pkgin. Tries the install with `pkgin -n` first, and stops when pkgin would remove packages
/// for it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pkgin {}

impl PackageProvider for Pkgin {
    fn name(&self) -> &str {
        "Pkgin"
    }

    fn available(&self) -> bool {
        match which("/usr/pkg/bin/pkgin") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "/usr/pkg/bin/pkgin is not available");
                false
            }
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self, _facts: &Facts) -> Vec<Step> {
        // TODO: Adjust for boot strapping pkgin
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![])
    }

    // TODO: Handle query pkgs with pkgin search
    fn query(&self, package: &PackageVariant) -> Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, _facts: &Facts) -> Result<Vec<Step>> {
        Ok(vec![
            Step::new(Exec {
                command: String::from("/usr/pkg/bin/pkgin"),
                arguments: vec![String::from("-n"), String::from("install")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                ..Default::default()
            })
            .as_root()
            .with_finalizer(StopIf(Box::new(OutputContains("removed")))),
            Step::new(Exec {
                command: String::from("/usr/pkg/bin/pkgin"),
                arguments: vec![String::from("-y"), String::from("install")]
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
