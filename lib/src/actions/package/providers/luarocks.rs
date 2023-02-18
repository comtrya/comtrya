use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::steps::Step;
use crate::{actions::package::PackageVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use which::which;
// use os_info;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LuaRocks {}

impl PackageProvider for LuaRocks {
    fn name(&self) -> &str {
        "LuaRocks"
    }

    fn available(&self) -> bool {
        match which("luarocks") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "luarocks is not available");
                false
            }
        }
    }

    #[instrument(name = "bootstrap", level = "info", skip(self))]
    fn bootstrap(&self) -> Vec<Step> {
        // TODO: Perhaps we can use the local OS provider package manager
        // to bootstrap?
        vec![]
    }

    fn has_repository(&self, _: &PackageRepository) -> bool {
        false
    }

    fn add_repository(&self, _: &PackageRepository) -> anyhow::Result<Vec<Step>> {
        Ok(vec![])
    }

    // TODO: Handle query pkgs with pkgin search
    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        // Install all packages for now, don't get smart about which
        // already are
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant) -> anyhow::Result<Vec<Step>> {
        let cli = match which("luarocks") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "LuaRocks is not available.");
                return Ok(vec![]);
            }
        };

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
                arguments: vec![String::from("install")]
                    .into_iter()
                    .chain(package.extra_args.clone())
                    .chain(package.packages())
                    .collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
