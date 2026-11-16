//! Getting a clone into the cache and keeping it up to date

use super::location::GitConfig;
use crate::atoms::{
    git::{clone_repository, Update},
    Atom,
};
use crate::manifests::providers::ManifestProviderError;
use crate::utilities::error_chain;

use std::path::Path;
use tracing::{info, warn};

/// Puts the old clone back if an update was killed between moving it away and moving the new
/// one in place
pub(super) fn recover_interrupted_update(cache_path: &Path) {
    let replaced = Update::replaced_dir(cache_path);
    if !cache_path.exists() && replaced.exists() {
        if let Err(err) = std::fs::rename(&replaced, cache_path) {
            warn!("Couldn't recover {}: {err}", replaced.display());
        }
    }
}

/// Brings the clone in `cache_path` to the remote's state. If that fails, say why and keep the
/// clone as it is: stale manifests are better than none when offline
pub(super) fn update_clone(cache_path: &Path, config: &GitConfig) {
    info!("Updating cached manifests.");
    let result = match gix::url::parse(config.repository.as_str()) {
        Ok(repository) => Atom::from(Update {
            repository,
            directory: cache_path.to_path_buf(),
            reference: config.branch.clone(),
        })
        .execute()
        .map(|_| ())
        .map_err(|err| error_chain(&err)),
        Err(err) => Err(error_chain(&err)),
    };

    if let Err(err) = result {
        warn!(
            "Couldn't update {}, using the cached clone in {}: {err}",
            config.repository,
            cache_path.display()
        );
    }
}

/// Clones the repository into `cache_path`, at the branch of the location if it has one
pub(super) fn fetch_and_clone(
    cache_path: &Path,
    config: &GitConfig,
) -> Result<(), ManifestProviderError> {
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

    clone_repository(&repository, cache_path, config.branch.as_deref(), true).map_err(|err| {
        ManifestProviderError::Clone {
            url: config.repository.clone(),
            message: error_chain(&err),
        }
    })?;

    info!("Finished fetch and clone operation.");

    Ok(())
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::super::test_repository::{clone, commit_file, repository, update};
    use super::*;
    use pretty_assertions::assert_eq;

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

    #[test]
    fn an_update_killed_before_the_new_clone_was_moved_in_is_recovered() {
        let cache = tempfile::tempdir().unwrap();
        let clone = cache.path().join("clone");
        let replaced = Update::replaced_dir(&clone);
        std::fs::create_dir(&replaced).unwrap();
        std::fs::write(replaced.join("main.yaml"), "actions: []\n").unwrap();

        recover_interrupted_update(&clone);

        assert!(clone.join("main.yaml").exists());
        assert!(!replaced.exists());
    }
}
