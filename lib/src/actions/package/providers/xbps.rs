use super::PackageProvider;
use crate::actions::package::repository::PackageRepository;
use crate::actions::package::PackageVariant;
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::runner::{self, CommandSpec, Runner};
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
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
        let output = self.runner.run(&CommandSpec::new("xbps-query").arg("-l"))?;
        if !output.success {
            return Err(anyhow!(
                "xbps-query -l failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        let list = String::from_utf8(output.stdout)?;
        let installed = installed_packages(&list);

        Ok(package
            .packages()
            .into_iter()
            .filter(|p| {
                if installed.contains(p.as_str()) {
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

/// The names of the installed packages in `xbps-query -l`, whose lines look like
/// `ii git-2.43.0_1  Git Tree History Storage Tool`. `ii` means installed, and the name is the
/// part of `<name>-<version>_<revision>` before the last `-`
fn installed_packages(list: &str) -> HashSet<&str> {
    list.lines()
        .filter_map(|line| line.strip_prefix("ii "))
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|pkgver| pkgver.rsplit_once('-'))
        .map(|(name, _version)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::fake::{fails, succeeds, FakeRunner};
    use pretty_assertions::assert_eq;

    /// Shortened from Void Linux
    const LIST: &str = "\
ii bash-5.2.037_1                     GNU Bourne Again Shell
ii ca-certificates-20250419+3.131.1_2 Common CA certificates for SSL/TLS from Mozilla
ii python3-yaml-6.0.2_2               YAML parser and emitter for Python3
uu half-installed-1.0_1               Unpacked, but not configured
";

    #[test]
    fn installed_packages_are_read_from_the_list() {
        assert_eq!(
            HashSet::from(["bash", "ca-certificates", "python3-yaml"]),
            installed_packages(LIST)
        );
    }

    #[test]
    fn query_returns_the_packages_that_are_not_installed() {
        let runner = FakeRunner::new()
            .answer("xbps-query -l", succeeds(LIST))
            .shared();
        let xbps = Xbps {
            runner: runner.clone(),
        };
        let package = PackageVariant {
            // "dash" matches the description of bash, "python3" is the start of python3-yaml
            list: ["bash", "python3-yaml", "dash", "python3", "half-installed"]
                .map(String::from)
                .to_vec(),
            ..Default::default()
        };

        assert_eq!(
            vec!["dash", "python3", "half-installed"],
            xbps.query(&package).unwrap()
        );
        assert_eq!(vec!["xbps-query -l"], runner.calls());
    }

    #[test]
    fn a_failing_query_is_an_error() {
        let xbps = Xbps {
            runner: FakeRunner::new()
                .answer("xbps-query -l", fails("", "ERROR: can't open pkgdb"))
                .shared(),
        };
        let package = PackageVariant {
            name: Some(String::from("bash")),
            ..Default::default()
        };

        let error = xbps.query(&package).unwrap_err();
        assert!(error.to_string().contains("can't open pkgdb"), "{error}");
    }
}
