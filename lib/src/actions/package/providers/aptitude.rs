//! apt on Debian, Ubuntu and their derivatives. Repositories are written as deb822 files,
//! PPAs are added with `add-apt-repository`

use super::PackageProvider;
use crate::actions::package::{
    repository::{PackageRepository, RepositoryKey},
    PackageVariant,
};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

/// apt, as `aptitude`, `apt` or `apt-get`. Always runs `apt install`, which skips installed packages
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Aptitude {}

impl Aptitude {
    /// Keeps apt and the package scripts from asking questions
    fn env(&self) -> Vec<(String, String)> {
        vec![(
            String::from("DEBIAN_FRONTEND"),
            String::from("noninteractive"),
        )]
    }

    /// Runs `command` with the privilege provider and without questions
    fn privileged_exec(&self, command: &str, arguments: Vec<String>, contexts: &Contexts) -> Step {
        let privilege_provider = utilities::get_privilege_provider(contexts);

        Step::new(Exec {
            command: String::from(command),
            arguments,
            environment: self.env(),
            privileged: true,
            privilege_provider,
            ..Default::default()
        })
    }

    /// Installs what comtrya itself needs, like `curl` for repository keys
    fn apt_install(&self, packages: &[&str], contexts: &Contexts) -> Step {
        self.privileged_exec(
            "apt",
            [String::from("install"), String::from("--yes")]
                .into_iter()
                .chain(packages.iter().map(|p| p.to_string()))
                .collect(),
            contexts,
        )
    }

    /// Reads the package lists again, after a repository was added
    fn apt_update(&self, contexts: &Contexts) -> Step {
        self.privileged_exec("apt", vec![String::from("update")], contexts)
    }

    /// Where the key of a repository is downloaded to, and what `Signed-By` points to
    fn key_path(key: &RepositoryKey) -> String {
        let key_name = key
            .name
            .clone()
            .unwrap_or_else(|| utilities::sha256(&key.url));
        format!("/usr/share/keyrings/{key_name}.asc")
    }
}

/// A repository in the one-line format `<uri> <suite> [<component>...]`,
/// e.g. `https://download.docker.com/linux/debian trixie stable`
struct DebRepository<'a> {
    /// Where the repository is, like `https://download.docker.com/linux/debian`
    uri: &'a str,
    /// The release, like `trixie`
    suite: &'a str,
    /// The parts of the release, like `main` or `stable`. Can be empty
    components: Vec<&'a str>,
}

impl<'a> DebRepository<'a> {
    /// Splits the one-line format, and fails without a URI and a suite
    fn parse(name: &'a str) -> anyhow::Result<Self> {
        let mut parts = name.split_whitespace();

        match (parts.next(), parts.next()) {
            (Some(uri), Some(suite)) => Ok(DebRepository {
                uri,
                suite,
                components: parts.collect(),
            }),
            _ => Err(anyhow!(
                "Invalid apt repository '{name}', expected '<uri> <suite> [<component>...]'"
            )),
        }
    }

    /// Path of the deb822 sources file, derived from the URI, so running again updates the same file
    fn sources_path(&self) -> String {
        let name = self
            .uri
            .split_once("://")
            .map_or(self.uri, |(_, rest)| rest)
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '.')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        format!("/etc/apt/sources.list.d/comtrya-{name}.sources")
    }

    /// The contents of the sources file
    fn deb822(&self, signed_by: Option<&str>) -> String {
        let mut contents = format!("Types: deb\nURIs: {}\nSuites: {}\n", self.uri, self.suite);

        if !self.components.is_empty() {
            contents.push_str(&format!("Components: {}\n", self.components.join(" ")));
        }

        if let Some(signed_by) = signed_by {
            contents.push_str(&format!("Signed-By: {signed_by}\n"));
        }

        contents
    }
}

impl PackageProvider for Aptitude {
    fn name(&self) -> &str {
        "Aptitude"
    }

    fn available(&self) -> bool {
        match which("apt") {
            Ok(_) => true,
            Err(_) => {
                warn!(message = "apt not available");
                false
            }
        }
    }

    fn bootstrap(&self, _contexts: &Contexts) -> Vec<Step> {
        // apt can't be installed by comtrya
        vec![]
    }

    fn has_repository(&self, repository: &PackageRepository) -> bool {
        if repository.name.starts_with("ppa:") {
            return false;
        }

        let Ok(deb) = DebRepository::parse(&repository.name) else {
            return false;
        };

        let signed_by = repository.key.as_ref().map(Aptitude::key_path);
        let key_present = signed_by
            .as_ref()
            .is_none_or(|path| std::path::Path::new(path).exists());

        key_present
            && std::fs::read_to_string(deb.sources_path())
                .is_ok_and(|contents| contents == deb.deb822(signed_by.as_deref()))
    }

    fn add_repository(
        &self,
        repository: &PackageRepository,
        contexts: &Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        // PPAs only exist on Ubuntu, which still ships add-apt-repository
        if repository.name.starts_with("ppa:") {
            if which("add-apt-repository").is_err() {
                steps.push(self.apt_install(&["software-properties-common"], contexts));
            }

            steps.push(self.privileged_exec(
                "add-apt-repository",
                vec![String::from("--yes"), repository.name.clone()],
                contexts,
            ));
            steps.push(self.apt_update(contexts));

            return Ok(steps);
        }

        let deb = DebRepository::parse(&repository.name)?;

        let signed_by = match &repository.key {
            Some(key) => {
                let key_path = Aptitude::key_path(key);

                if which("curl").is_err() {
                    steps.push(self.apt_install(&["curl", "ca-certificates"], contexts));
                }

                steps.push(self.privileged_exec(
                    "curl",
                    vec![
                        String::from("-fsSL"),
                        String::from("-o"),
                        key_path.clone(),
                        key.url.clone(),
                    ],
                    contexts,
                ));

                Some(key_path)
            }
            None => None,
        };

        // Write a deb822 .sources file, the format apt recommends since Debian 13.
        // The contents and path are passed as arguments, so the shell never interprets them.
        steps.push(self.privileged_exec(
            "sh",
            vec![
                String::from("-c"),
                String::from(r#"printf '%s' "$1" > "$2""#),
                String::from("sh"),
                deb.deb822(signed_by.as_deref()),
                deb.sources_path(),
            ],
            contexts,
        ));
        steps.push(self.apt_update(contexts));

        Ok(steps)
    }

    fn query(&self, package: &PackageVariant) -> anyhow::Result<Vec<String>> {
        Ok(package.packages())
    }

    fn install(&self, package: &PackageVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![self.privileged_exec(
            "apt",
            [String::from("install"), String::from("--yes")]
                .into_iter()
                .chain(package.extra_args.clone())
                .chain(package.packages())
                .collect(),
            contexts,
        )])
    }
}

#[cfg(test)]
mod test {
    use crate::actions::package::repository::RepositoryKey;
    use crate::contexts::Contexts;
    use pretty_assertions::assert_eq;

    use super::*;

    fn add_repository(repository: PackageRepository) -> Vec<String> {
        Aptitude {}
            .add_repository(&repository, &Contexts::default())
            .unwrap()
            .iter()
            .map(|step| step.atom.to_string())
            .collect()
    }

    #[test]
    fn test_parse_repository() {
        let deb =
            DebRepository::parse("https://download.docker.com/linux/debian trixie stable").unwrap();

        assert_eq!("https://download.docker.com/linux/debian", deb.uri);
        assert_eq!("trixie", deb.suite);
        assert_eq!(vec!["stable"], deb.components);
        assert_eq!(
            "/etc/apt/sources.list.d/comtrya-download.docker.com-linux-debian.sources",
            deb.sources_path()
        );

        assert!(DebRepository::parse("https://example.com").is_err());
    }

    #[test]
    fn test_deb822() {
        let deb = DebRepository::parse("https://example.com/debian trixie main contrib").unwrap();

        assert_eq!(
            "Types: deb\nURIs: https://example.com/debian\nSuites: trixie\nComponents: main contrib\nSigned-By: /usr/share/keyrings/example.asc\n",
            deb.deb822(Some("/usr/share/keyrings/example.asc"))
        );

        // Flat repositories have no components
        let deb = DebRepository::parse("https://example.com/debian ./").unwrap();
        assert_eq!(
            "Types: deb\nURIs: https://example.com/debian\nSuites: ./\n",
            deb.deb822(None)
        );
    }

    #[test]
    fn test_add_repository_without_key() {
        let steps = add_repository(PackageRepository {
            name: String::from("https://example.com/debian trixie main"),
            ..Default::default()
        });

        assert_eq!(2, steps.len());
        assert!(steps[0].contains("sh -c"));
        assert!(steps[0].contains("URIs: https://example.com/debian"));
        assert!(steps[0].contains("/etc/apt/sources.list.d/comtrya-example.com-debian.sources"));
        assert!(!steps[0].contains("Signed-By"));
        assert!(steps[1].contains("apt update"));
    }

    #[test]
    fn test_add_repository_with_key() {
        let steps = add_repository(PackageRepository {
            name: String::from("https://example.com/debian trixie main"),
            key: Some(RepositoryKey {
                url: String::from("https://example.com/key.asc"),
                name: Some(String::from("example")),
                ..Default::default()
            }),
            ..Default::default()
        });

        let n = steps.len();
        assert!(steps[n - 3].contains("curl -fsSL -o /usr/share/keyrings/example.asc"));
        assert!(steps[n - 2].contains("Signed-By: /usr/share/keyrings/example.asc"));
        assert!(steps[n - 1].contains("apt update"));
    }

    #[test]
    fn test_regression_share_ring() {
        let steps = add_repository(PackageRepository {
            name: String::from("https://example.com/debian trixie main"),
            key: Some(RepositoryKey {
                url: String::from("abc"),
                fingerprint: Some(String::from("abc")),
                ..Default::default()
            }),
            ..Default::default()
        });

        assert!(steps
            .iter()
            .any(|step| step.contains(" /usr/share/keyrings/")));
    }

    #[test]
    fn test_add_repository_ppa() {
        let steps = add_repository(PackageRepository {
            name: String::from("ppa:neovim-ppa/stable"),
            ..Default::default()
        });

        let n = steps.len();
        assert!(steps[n - 2].contains("add-apt-repository --yes ppa:neovim-ppa/stable"));
        assert!(steps[n - 1].contains("apt update"));
    }

    #[test]
    fn test_add_repository_invalid() {
        assert!(Aptitude {}
            .add_repository(
                &PackageRepository {
                    name: String::from("test"),
                    ..Default::default()
                },
                &Contexts::default(),
            )
            .is_err());
    }
}
