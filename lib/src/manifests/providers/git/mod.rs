//! Manifests in git repositories. A repository is cloned into the cache once per branch, and
//! updated on every run, as far as the remote is reachable

mod cache;
mod clone;
mod location;
#[cfg(test)]
#[cfg(unix)]
mod test_repository;

pub use cache::git_cache_dir;

use super::{ManifestProvider, ManifestProviderError};
use location::GitConfig;

/// Clones `https://`, `git://` and `ssh://` URLs
#[derive(Debug)]
pub struct GitManifestProvider;

impl ManifestProvider for GitManifestProvider {
    fn looks_familiar(&self, url: &str) -> bool {
        use regex::Regex;

        if let Ok(regex) = Regex::new(r"^(https|git|ssh)://") {
            regex.is_match(url)
        } else {
            false
        }
    }

    fn resolve(&self, url: &str) -> Result<std::path::PathBuf, ManifestProviderError> {
        let config = GitConfig::parse(url);
        let cache_path = git_cache_dir()
            .ok_or(ManifestProviderError::NoCacheDir)?
            .join(cache::cache_name(&config));
        clone::recover_interrupted_update(&cache_path);
        if cache_path.exists() {
            clone::update_clone(&cache_path, &config);
        } else {
            clone::fetch_and_clone(&cache_path, &config)?;
        }

        Ok(config.manifest_dir(cache_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_familiar_for_git_urls_only() {
        let provider = GitManifestProvider;

        assert!(provider.looks_familiar("https://github.com/comtrya/comtrya"));
        assert!(provider.looks_familiar("git://github.com/comtrya/comtrya"));
        assert!(provider.looks_familiar("ssh://git@github.com/comtrya/comtrya"));
        assert!(!provider.looks_familiar("./manifests"));
        assert!(!provider.looks_familiar("/home/user/manifests"));
    }
}
