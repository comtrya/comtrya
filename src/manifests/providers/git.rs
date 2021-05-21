use super::{ManifestProvider, ManifestProviderError};

use gitsync::GitSync;
use tracing::{error, info};

#[derive(Debug)]
pub struct GitManifestProvider;

impl ManifestProvider for GitManifestProvider {
    fn looks_familiar(&self, url: &str) -> bool {
        use regex::Regex;
        let regex = Regex::new(r"^(https|git|ssh)://").unwrap();

        regex.is_match(url)
    }

    fn resolve(&self, url: &str) -> Result<std::path::PathBuf, super::ManifestProviderError> {
        // Extract this to a function!
        let clean_repo_url = self.clean_git_url(&url);
        let cache_path = dirs_next::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/comtrya"))
            .join("comtrya")
            .join("manifests")
            .join("git")
            .join(clean_repo_url);

        let git_sync = GitSync {
            repo: url.to_string(),
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

        Ok(cache_path)
    }
}

impl GitManifestProvider {
    fn clean_git_url(&self, uri: &str) -> String {
        uri.to_string()
            .replace("https", "")
            .replace("http", "")
            .replace(":", "")
            .replace(".", "")
            .replace("/", "")
    }
}

#[cfg(test)]
mod test {
    use super::super::ManifestProvider;
    use super::GitManifestProvider;

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
