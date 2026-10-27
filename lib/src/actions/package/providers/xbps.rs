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
use tracing::{debug, trace, warn};
use which::which;

#[derive(Clone)]
pub struct Xbps {
    runner: Arc<dyn Runner>,
}

impl Default for Xbps {
    fn default() -> Self {
        Self {
            runner: runner::system(),
        }
    }
}

impl PackageProvider for Xbps {
    fn name(&self) -> &str {
        "Xbps"
    }

    fn available(&self) -> bool {
        let mut install = false;
        let mut query = false;
        match which("xbps-install") {
            Ok(_) => {
                install = true;
            }
            Err(_) => {
                warn!(message = "xbps-install not available");
            }
        };
        match which("xbps-query") {
            Ok(_) => {
                query = true;
            }
            Err(_) => {
                warn!(message = "xbps-query not available");
            }
        };
        install && query
    }

    fn bootstrap(&self, _contexts: &Contexts) -> Vec<Step> {
        vec![]
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
        let command = CommandSpec::new("xbps-query")
            .arg("-s")
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
        let need_installed = self.query(package)?;
        if need_installed.is_empty() {
            return Ok(vec![]);
        }

        let privilege_provider = utilities::get_privilege_provider(contexts);

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("xbps-install"),
                arguments: [
                    vec![
                        String::from("-S"),
                        String::from("--yes"),
                        String::from("--update"),
                    ],
                    package.extra_args.clone(),
                    need_installed,
                ]
                .concat(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
