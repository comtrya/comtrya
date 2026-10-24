use super::{ManifestProvider, ManifestProviderError};

use gix;
use gix::interrupt;
use gix::progress::Discard;

use dirs_next;

use tracing::info;

#[derive(Debug)]
pub struct GitManifestProvider;

/// Directory where manifests fetched from git repositories are cloned to,
/// one subdirectory per repository
pub fn git_cache_dir() -> Option<std::path::PathBuf> {
    dirs_next::cache_dir().map(|dir| dir.join("comtrya").join("manifests").join("git"))
}

#[derive(Debug, PartialEq)]
pub(crate) struct GitConfig {
    repository: String,
    branch: Option<String>,
    path: Option<String>,
}

impl ManifestProvider for GitManifestProvider {
    fn looks_familiar(&self, url: &str) -> bool {
        use regex::Regex;

        if let Ok(regex) = Regex::new(r"^(https|git|ssh)://") {
            regex.is_match(url)
        } else {
            false
        }
    }

    fn resolve(
        &self,
        url: &str,
    ) -> anyhow::Result<std::path::PathBuf, super::ManifestProviderError> {
        let config = self.parse_config_url(url);
        let clean_url = self.clean_git_url(&config.repository);
        let cache_path = git_cache_dir()
            .ok_or(ManifestProviderError::NoResolution)?
            .join(clean_url);
        if !cache_path.exists() {
            self.fetch_and_clone(&cache_path, &config)?;
        }

        Ok(cache_path)
    }
}

impl GitManifestProvider {
    fn fetch_and_clone(
        &self,
        cache_path: &std::path::Path,
        config: &GitConfig,
    ) -> anyhow::Result<(), super::ManifestProviderError> {
        info!("Preparing to fetch and clone manifests.");
        let r = std::fs::create_dir_all(cache_path);
        if r.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = gix::url::parse(config.repository.as_str());

        if url.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = url.unwrap();

        unsafe {
            let handler = interrupt::init_handler(1, || {});
            if handler.is_err() {
                return Err(ManifestProviderError::NoResolution);
            }
        };

        let prepare_clone = gix::prepare_clone(url.clone(), cache_path);
        if prepare_clone.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_clone = prepare_clone.unwrap();

        let prepare_checkout =
            prepare_clone.fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED);
        if prepare_checkout.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_checkout = prepare_checkout.unwrap().0;

        let repo = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED);
        if repo.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }
        let repo = repo.unwrap().0;

        let success = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .expect("always present after clone");
        if success.is_err() {
            return Err(ManifestProviderError::NoResolution);
        }

        info!("Finished fetch and clone operation.");

        Ok(())
    }

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
            .replace([':', '.', '/'], "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn cache_dir_is_inside_the_platform_cache_dir() {
        let cache_dir = git_cache_dir().unwrap();

        assert!(cache_dir.starts_with(dirs_next::cache_dir().unwrap()));
        assert!(cache_dir.ends_with(
            std::path::Path::new("comtrya")
                .join("manifests")
                .join("git")
        ));
    }

    #[test]
    fn looks_familiar_for_git_urls_only() {
        let provider = GitManifestProvider;

        assert!(provider.looks_familiar("https://github.com/comtrya/comtrya"));
        assert!(provider.looks_familiar("git://github.com/comtrya/comtrya"));
        assert!(provider.looks_familiar("ssh://git@github.com/comtrya/comtrya"));
        assert!(!provider.looks_familiar("./manifests"));
        assert!(!provider.looks_familiar("/home/user/manifests"));
    }

    #[test]
    fn parse_config_url_variants() {
        let provider = GitManifestProvider;
        let repository = String::from("https://github.com/comtrya/comtrya");

        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: None,
                path: None,
            },
            provider.parse_config_url("https://github.com/comtrya/comtrya")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: Some(String::from("main")),
                path: None,
            },
            provider.parse_config_url("https://github.com/comtrya/comtrya#main")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: Some(String::from("main")),
                path: Some(String::from("examples")),
            },
            provider.parse_config_url("https://github.com/comtrya/comtrya#main:examples")
        );
        assert_eq!(
            GitConfig {
                repository: repository.clone(),
                branch: None,
                path: Some(String::from("examples")),
            },
            provider.parse_config_url("https://github.com/comtrya/comtrya#:examples")
        );
        assert_eq!(
            GitConfig {
                repository,
                branch: Some(String::from("main")),
                path: None,
            },
            provider.parse_config_url("https://github.com/comtrya/comtrya#main:")
        );
    }

    #[test]
    fn clean_git_url_is_a_single_path_segment() {
        let provider = GitManifestProvider;

        assert_eq!(
            "githubcomcomtryacomtrya",
            provider.clean_git_url("https://github.com/comtrya/comtrya")
        );
    }
}
