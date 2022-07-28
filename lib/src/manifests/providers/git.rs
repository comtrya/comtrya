use super::{ManifestProvider, ManifestProviderError};

use gitsync::GitSync;
use tracing::{error, info};

#[derive(Debug)]
pub struct GitManifestProvider;

#[derive(Debug, PartialEq)]
pub(crate) struct GitConfig {
    repository: String,
    branch: Option<String>,
    path: Option<String>,
}

impl ManifestProvider for GitManifestProvider {
    fn looks_familiar(&self, url: &str) -> bool {
        use regex::Regex;
        let regex = Regex::new(r"^(https|git|ssh)://").unwrap();

        regex.is_match(url)
    }

    fn resolve(&self, url: &str) -> Result<std::path::PathBuf, super::ManifestProviderError> {
        let git_config = self.parse_config_url(url);
        let clean_repo_url = self.clean_git_url(&git_config.repository);
        let cache_path = dirs_next::cache_dir()
            .unwrap()
            .join("comtrya")
            .join("manifests")
            .join("git")
            .join(clean_repo_url);

        let git_sync = GitSync {
            repo: git_config.repository,
            branch: git_config.branch,
            dir: cache_path.clone(),
            ..Default::default()
        };

        info!(
            "Syncing Git repository {} to {}",
            &url,
            cache_path.to_str().unwrap()
        );

        if let Err(error) = git_sync.bootstrap() {
            error!("Failed to bootstrap repository, {:?}", error);
            return Err(ManifestProviderError::NoResolution);
        }

        if let Err(error) = git_sync.sync() {
            error!("Failed to bootstrap repository, {:?}", error);
            return Err(ManifestProviderError::NoResolution);
        }

        Ok(cache_path.join(git_config.path.unwrap_or_else(|| String::from(""))))
    }
}

impl GitManifestProvider {
    fn parse_config_url(&self, uri: &str) -> GitConfig {
        let (repository, parts) = match uri.split_once('#') {
            Some(parts) => parts,
            None => {
                return GitConfig {
                    repository: String::from(uri),
                    branch: None,
                    path: None,
                };
            }
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

    fn clean_git_url(&self, uri: &str) -> String {
        uri.to_string()
            .replace("https", "")
            .replace("http", "")
            .replace(':', "")
            .replace('.', "")
            .replace('/', "")
    }
}

// Need to work out why this doesn't pass on Windows.
#[cfg(test)]
#[cfg(unix)]
mod test {
    use crate::manifests::providers::git::GitConfig;

    use super::super::ManifestProvider;
    use super::GitManifestProvider;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_clean_git_url() {
        let git_manifest_provider = GitManifestProvider {};

        assert_eq!(
            "githubcomcomtryacomtrya",
            git_manifest_provider.clean_git_url("https://github.com/comtrya/comtrya")
        );
    }

    #[test]
    fn test_looks_familiar() {
        let git_manifest_provider = GitManifestProvider {};

        assert_eq!(
            true,
            git_manifest_provider
                .looks_familiar(&String::from("https://github.com/comtrya/comtrya"))
        );

        assert_eq!(
            true,
            git_manifest_provider.looks_familiar(&String::from("git://github.com/comtrya/comtrya"))
        );

        assert_eq!(
            true,
            git_manifest_provider.looks_familiar(&String::from("ssh://github.com/comtrya/comtrya"))
        );

        assert_eq!(
            false,
            git_manifest_provider.looks_familiar(&String::from("/github.com/comtrya/comtrya"))
        );

        assert_eq!(
            false,
            git_manifest_provider.looks_familiar(&String::from("github.com/comtrya/comtrya"))
        );
    }

    #[test]
    fn test_parse_config_url() {
        let git_manifest_provider = GitManifestProvider {};

        assert_eq!(
            GitConfig {
                repository: String::from("https://hubgit.com/comtrya/comtrya"),
                branch: None,
                path: None,
            },
            git_manifest_provider.parse_config_url("https://hubgit.com/comtrya/comtrya")
        );

        assert_eq!(
            GitConfig {
                repository: String::from("https://hubgit.com/comtrya/comtrya"),
                branch: Some(String::from("main")),
                path: None,
            },
            git_manifest_provider.parse_config_url("https://hubgit.com/comtrya/comtrya#main")
        );

        assert_eq!(
            GitConfig {
                repository: String::from("https://hubgit.com/comtrya/comtrya"),
                branch: Some(String::from("main")),
                path: Some(String::from("comtrya")),
            },
            git_manifest_provider
                .parse_config_url("https://hubgit.com/comtrya/comtrya#main:comtrya")
        );

        assert_eq!(
            GitConfig {
                repository: String::from("https://hubgit.com/comtrya/comtrya"),
                branch: Some(String::from("main")),
                path: None,
            },
            git_manifest_provider.parse_config_url("https://hubgit.com/comtrya/comtrya#main:")
        );

        assert_eq!(
            GitConfig {
                repository: String::from("https://hubgit.com/comtrya/comtrya"),
                branch: None,
                path: Some(String::from("comtrya")),
            },
            git_manifest_provider.parse_config_url("https://hubgit.com/comtrya/comtrya#:comtrya")
        );
    }

    #[test]
    fn test_resolve() {
        let git_manifest_provider = GitManifestProvider {};

        assert_eq!(
            true,
            git_manifest_provider
                .resolve(&String::from("https://github.com/comtrya/comtrya"))
                .is_ok()
        );

        assert_eq!(
            true,
            git_manifest_provider
                .resolve(&String::from("https://hubgit.com/comtrya/comtrya"))
                .is_err()
        );
    }
}
