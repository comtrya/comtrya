//! A git location as the user writes it: `<url>[#<branch>][:<path>]`

use std::path::PathBuf;

/// A git location: `<url>[#<branch>][:<path>]`
#[derive(Debug, PartialEq)]
pub(super) struct GitConfig {
    /// The URL of the repository
    pub(super) repository: String,
    /// The branch or tag to check out, the remote's `HEAD` if `None`
    pub(super) branch: Option<String>,
    /// The directory of the manifests in the repository, its root if `None`
    pub(super) path: Option<String>,
}

impl GitConfig {
    /// Splits a location into URL, branch and path
    pub(super) fn parse(uri: &str) -> GitConfig {
        let Some((repository, parts)) = uri.split_once('#') else {
            return GitConfig {
                repository: String::from(uri),
                branch: None,
                path: None,
            };
        };

        let (reference, path) = match parts.split_once(':') {
            Some(("", path)) => (None, Some(path.to_string())),
            Some((reference, "")) => (Some(reference.to_string()), None),
            Some((reference, path)) => (Some(reference.to_string()), Some(path.to_string())),
            None => {
                return GitConfig {
                    repository: String::from(repository),
                    branch: Some(parts.to_string()),
                    path: None,
                }
            }
        };

        GitConfig {
            repository: String::from(repository),
            branch: reference,
            path,
        }
    }

    /// Where the manifests are in the clone at `clone`: `path` in it, if there is one
    pub(super) fn manifest_dir(&self, clone: PathBuf) -> PathBuf {
        match &self.path {
            Some(path) => clone.join(path),
            None => clone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parse_config_url_variants() {
        let repository = String::from("https://github.com/comtrya/comtrya");

        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: None,
                path: None,
            },
            GitConfig::parse("https://github.com/comtrya/comtrya")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: Some(String::from("main")),
                path: None,
            },
            GitConfig::parse("https://github.com/comtrya/comtrya#main")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: Some(String::from("main")),
                path: Some(String::from("examples")),
            },
            GitConfig::parse("https://github.com/comtrya/comtrya#main:examples")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: None,
                path: Some(String::from("examples")),
            },
            GitConfig::parse("https://github.com/comtrya/comtrya#:examples")
        );
        assert_eq!(
            GitConfig {
                repository,
                branch: Some(String::from("main")),
                path: None,
            },
            GitConfig::parse("https://github.com/comtrya/comtrya#main:")
        );
    }
}
