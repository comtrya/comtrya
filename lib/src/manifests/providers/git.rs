use super::{ManifestProvider, ManifestProviderError};

use gix;
use gix::interrupt;
use gix::progress::Discard;

use dirs_next;

use tracing::info;

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
        let config = self.parse_config_url(&url);
        let clean_url = self.clean_git_url(&config.repository);
        let cache_path = dirs_next::cache_dir()
            .ok_or(ManifestProviderError::NoResolution)?
            .join("comtrya")
            .join("manifests")
            .join("git")
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
        cache_path: &std::path::PathBuf,
        config: &GitConfig,
    ) -> anyhow::Result<(), super::ManifestProviderError> {
	info!("Preparing to fetch and clone manifests.");
        let r = std::fs::create_dir_all(cache_path.clone());
        if let Err(_) = r {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = gix::url::parse(config.repository.clone().as_str().into());

        if let Err(_) = url {
            return Err(ManifestProviderError::NoResolution);
        }

        let url = url.unwrap();

        unsafe {
            let handler = interrupt::init_handler(1, || {});
            if let Err(_) = handler {
                return Err(ManifestProviderError::NoResolution);
            }
        };

        let prepare_clone = gix::prepare_clone(url.clone(), cache_path.clone());
        if let Err(_) = prepare_clone {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_clone = prepare_clone.unwrap();

        let prepare_checkout =
            prepare_clone.fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED);
        if let Err(_) = prepare_checkout {
            return Err(ManifestProviderError::NoResolution);
        }
        let mut prepare_checkout = prepare_checkout.unwrap().0;

        let repo = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED);
        if let Err(_) = repo {
            return Err(ManifestProviderError::NoResolution);
        }
        let repo = repo.unwrap().0;

        let success = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .expect("always present after clone");
        if let Err(_) = success {
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
