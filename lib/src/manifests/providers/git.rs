//! Manifests in git repositories. A repository is cloned into the cache once per branch, and
//! updated on every run, as far as the remote is reachable

use super::{ManifestProvider, ManifestProviderError};

use crate::atoms::{
    git::{clone_repository, Update},
    Atom,
};

use std::path::PathBuf;
use tracing::{info, warn};

/// Clones `https://`, `git://` and `ssh://` URLs
#[derive(Debug)]
pub struct GitManifestProvider;

/// Directory where manifests fetched from git repositories are cloned to,
/// one subdirectory per repository and branch
pub fn git_cache_dir() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("comtrya").join("manifests").join("git"))
}

/// A git location: `<url>[#<branch>][:<path>]`
#[derive(Debug, PartialEq)]
pub(crate) struct GitConfig {
    /// The URL of the repository
    repository: String,
    /// The branch or tag to check out, the remote's `HEAD` if `None`
    branch: Option<String>,
    /// The directory of the manifests in the repository, its root if `None`
    path: Option<String>,
}

impl GitConfig {
    /// Where the manifests are in the clone at `clone`: `path` in it, if there is one
    fn manifest_dir(&self, clone: PathBuf) -> PathBuf {
        match &self.path {
            Some(path) => clone.join(path),
            None => clone,
        }
    }
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
        let cache_path = git_cache_dir()
            .ok_or(ManifestProviderError::NoCacheDir)?
            .join(self.cache_name(&config));
        Self::recover_interrupted_update(&cache_path);
        if cache_path.exists() {
            self.update_clone(&cache_path, &config);
        } else {
            self.fetch_and_clone(&cache_path, &config)?;
        }

        Ok(config.manifest_dir(cache_path))
    }
}

impl GitManifestProvider {
    /// Puts the old clone back if an update was killed between moving it away and moving the
    /// new one in place
    fn recover_interrupted_update(cache_path: &std::path::Path) {
        let replaced = Update::replaced_dir(cache_path);
        if !cache_path.exists() && replaced.exists() {
            if let Err(err) = std::fs::rename(&replaced, cache_path) {
                warn!("Couldn't recover {}: {err}", replaced.display());
            }
        }
    }

    /// Brings the clone in `cache_path` to the remote's state. If that fails, say why and keep
    /// the clone as it is: stale manifests are better than none when offline
    fn update_clone(&self, cache_path: &std::path::Path, config: &GitConfig) {
        info!("Updating cached manifests.");
        let result = gix::url::parse(config.repository.as_str())
            .map_err(anyhow::Error::from)
            .and_then(|repository| {
                Atom::from(Update {
                    repository,
                    directory: cache_path.to_path_buf(),
                    reference: config.branch.clone(),
                })
                .execute()
            });

        if let Err(err) = result {
            warn!(
                "Couldn't update {}, using the cached clone in {}: {err:#}",
                config.repository,
                cache_path.display()
            );
        }
    }

    /// Clones the repository into `cache_path`, at the branch of the location if it has one
    fn fetch_and_clone(
        &self,
        cache_path: &std::path::Path,
        config: &GitConfig,
    ) -> anyhow::Result<(), super::ManifestProviderError> {
        info!("Preparing to fetch and clone manifests.");
        std::fs::create_dir_all(cache_path).map_err(|source| ManifestProviderError::CacheDir {
            path: cache_path.to_path_buf(),
            source,
        })?;

        let repository = gix::url::parse(config.repository.as_str()).map_err(|err| {
            ManifestProviderError::InvalidUrl {
                url: config.repository.clone(),
                message: err.to_string(),
            }
        })?;

        clone_repository(&repository, cache_path, config.branch.as_deref(), true).map_err(
            |err| ManifestProviderError::Clone {
                url: config.repository.clone(),
                message: format!("{err:#}"),
            },
        )?;

        info!("Finished fetch and clone operation.");

        Ok(())
    }

    /// Splits a location into URL, branch and path
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

    /// The name of the clone's directory in the cache: one per repository and branch
    fn cache_name(&self, config: &GitConfig) -> String {
        let repository = self.clean_git_url(&config.repository);
        match &config.branch {
            // Branch names may contain `/`, which would nest the directory
            Some(branch) => format!("{repository}#{}", branch.replace(['/', '\\', ':'], "-")),
            None => repository,
        }
    }

    /// The repository URL as a single path segment
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

        assert!(cache_dir.starts_with(dirs::cache_dir().unwrap()));
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

    /// A repository with `main.yaml` on `main`, and `sub/other.yaml` on `other`
    #[cfg(unix)]
    fn repository() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let git = |args: &[&str]| {
            // Without the user's settings, like signing commits, which may wait for a passphrase
            let status = std::process::Command::new("git")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .args([
                    "-c",
                    "user.name=comtrya",
                    "-c",
                    "user.email=comtrya@example.com",
                ])
                .args(args)
                .current_dir(dir.path())
                .output()
                .unwrap()
                .status;
            assert!(status.success(), "git {args:?}");
        };

        git(&["init", "--quiet", "--initial-branch", "main"]);
        std::fs::write(dir.path().join("main.yaml"), "actions: []\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", "main"]);
        git(&["checkout", "--quiet", "-b", "other"]);
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/other.yaml"), "actions: []\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "--quiet", "-m", "other"]);
        git(&["checkout", "--quiet", "main"]);
        dir
    }

    /// Clones `location` like `resolve`, but into `cache`, and returns the manifest directory
    #[cfg(unix)]
    fn clone(location: &str, cache: &std::path::Path) -> Result<PathBuf, ManifestProviderError> {
        let provider = GitManifestProvider;
        let config = provider.parse_config_url(location);
        let clone = cache.join(provider.cache_name(&config));
        provider.fetch_and_clone(&clone, &config)?;
        Ok(config.manifest_dir(clone))
    }

    #[cfg(unix)]
    #[test]
    fn the_branch_and_path_of_a_location_are_used() {
        let repository = repository();
        let cache = tempfile::tempdir().unwrap();
        let url = repository.path().display().to_string();

        let manifests = clone(&format!("{url}#other:sub"), cache.path()).unwrap();
        assert!(manifests.join("other.yaml").exists(), "{manifests:?}");

        let manifests = clone(&url, cache.path()).unwrap();
        assert!(manifests.join("main.yaml").exists(), "{manifests:?}");
        assert!(!manifests.join("sub").exists());
    }

    #[test]
    fn each_branch_has_its_own_clone() {
        let provider = GitManifestProvider;
        let name = |location: &str| provider.cache_name(&provider.parse_config_url(location));

        assert_eq!(
            "githubcomcomtryacomtrya",
            name("https://github.com/comtrya/comtrya")
        );
        assert_eq!(
            name("https://github.com/comtrya/comtrya#:examples"),
            name("https://github.com/comtrya/comtrya")
        );
        assert_ne!(
            name("https://github.com/comtrya/comtrya#feat/x"),
            name("https://github.com/comtrya/comtrya")
        );
        assert!(!name("https://github.com/comtrya/comtrya#feat/x").contains('/'));
    }

    #[test]
    fn clean_git_url_is_a_single_path_segment() {
        let provider = GitManifestProvider;

        assert_eq!(
            "githubcomcomtryacomtrya",
            provider.clean_git_url("https://github.com/comtrya/comtrya")
        );
    }

    /// Runs git in `dir` without the user's settings
    #[cfg(unix)]
    fn git_in(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args([
                "-c",
                "user.name=comtrya",
                "-c",
                "user.email=comtrya@example.com",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
            .status;
        assert!(status.success(), "git {args:?}");
    }

    /// Commits a file `name` on `branch` of the repository in `dir`, and goes back to `main`
    #[cfg(unix)]
    fn commit_file(dir: &std::path::Path, branch: &str, name: &str) {
        git_in(dir, &["checkout", "--quiet", branch]);
        std::fs::write(dir.join(name), "actions: []\n").unwrap();
        git_in(dir, &["add", "."]);
        git_in(dir, &["commit", "--quiet", "-m", name]);
        git_in(dir, &["checkout", "--quiet", "main"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_cached_clone_gets_new_commits_of_the_remote() {
        let repository = repository();
        let cache = tempfile::tempdir().unwrap();
        let url = repository.path().display().to_string();
        let manifests = clone(&url, cache.path()).unwrap();
        assert!(!manifests.join("new.yaml").exists());

        commit_file(repository.path(), "main", "new.yaml");
        update(&url, cache.path());

        assert!(manifests.join("new.yaml").exists());
        assert!(manifests.join("main.yaml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_cached_clone_of_a_branch_gets_new_commits_of_the_branch() {
        let repository = repository();
        let cache = tempfile::tempdir().unwrap();
        let location = format!("{}#other:sub", repository.path().display());
        let manifests = clone(&location, cache.path()).unwrap();

        commit_file(repository.path(), "other", "sub/new.yaml");
        update(&location, cache.path());

        assert!(manifests.join("new.yaml").exists());
        assert!(manifests.join("other.yaml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn local_changes_and_removed_files_of_a_cached_clone_are_discarded() {
        let repository = repository();
        let cache = tempfile::tempdir().unwrap();
        let url = repository.path().display().to_string();
        let manifests = clone(&url, cache.path()).unwrap();
        std::fs::write(manifests.join("main.yaml"), "changed\n").unwrap();
        std::fs::write(manifests.join("local.yaml"), "local\n").unwrap();

        update(&url, cache.path());

        assert_eq!(
            "actions: []\n",
            std::fs::read_to_string(manifests.join("main.yaml")).unwrap()
        );
        assert!(!manifests.join("local.yaml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_cached_clone_is_used_when_the_remote_is_gone() {
        let repository = repository();
        let cache = tempfile::tempdir().unwrap();
        let url = repository.path().display().to_string();
        let manifests = clone(&url, cache.path()).unwrap();
        drop(repository);

        update(&url, cache.path());

        assert!(manifests.join("main.yaml").exists());
        let leftovers = std::fs::read_dir(cache.path()).unwrap().count();
        assert_eq!(1, leftovers, "temporary directories are cleaned up");
    }

    #[cfg(unix)]
    #[test]
    fn manifest_clones_have_only_the_latest_commit() {
        let repository = repository();
        commit_file(repository.path(), "main", "second.yaml");
        let cache = tempfile::tempdir().unwrap();
        // `file://` because git only does shallow clones over a real transport
        let url = format!("file://{}", repository.path().display());
        let manifests = clone(&url, cache.path()).unwrap();
        assert!(manifests.join("second.yaml").exists());
        assert!(manifests.join(".git/shallow").exists());

        commit_file(repository.path(), "main", "third.yaml");
        update(&url, cache.path());
        assert!(manifests.join("third.yaml").exists());
        assert!(manifests.join(".git/shallow").exists());
    }

    #[cfg(unix)]
    #[test]
    fn an_update_killed_before_the_new_clone_was_moved_in_is_recovered() {
        let cache = tempfile::tempdir().unwrap();
        let clone = cache.path().join("clone");
        let replaced = Update::replaced_dir(&clone);
        std::fs::create_dir(&replaced).unwrap();
        std::fs::write(replaced.join("main.yaml"), "actions: []\n").unwrap();

        GitManifestProvider::recover_interrupted_update(&clone);

        assert!(clone.join("main.yaml").exists());
        assert!(!replaced.exists());
    }

    /// Updates the clone of `location` in `cache`, like `resolve` does for an existing clone
    #[cfg(unix)]
    fn update(location: &str, cache: &std::path::Path) {
        let provider = GitManifestProvider;
        let config = provider.parse_config_url(location);
        provider.update_clone(&cache.join(provider.cache_name(&config)), &config);
    }
}
