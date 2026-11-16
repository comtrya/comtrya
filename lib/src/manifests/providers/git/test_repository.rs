//! A local repository to clone from, and helpers that clone and update like `resolve` does

use super::cache::cache_name;
use super::clone::{fetch_and_clone, update_clone};
use super::location::GitConfig;
use crate::manifests::providers::ManifestProviderError;

use std::path::{Path, PathBuf};

/// Runs git in `dir` without the user's settings, like signing commits, which may wait for a
/// passphrase
fn git_in(dir: &Path, args: &[&str]) {
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

/// A repository with `main.yaml` on `main`, and `sub/other.yaml` on `other`
pub(super) fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();

    git_in(dir.path(), &["init", "--quiet", "--initial-branch", "main"]);
    std::fs::write(dir.path().join("main.yaml"), "actions: []\n").unwrap();
    git_in(dir.path(), &["add", "."]);
    git_in(dir.path(), &["commit", "--quiet", "-m", "main"]);
    git_in(dir.path(), &["checkout", "--quiet", "-b", "other"]);
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/other.yaml"), "actions: []\n").unwrap();
    git_in(dir.path(), &["add", "."]);
    git_in(dir.path(), &["commit", "--quiet", "-m", "other"]);
    git_in(dir.path(), &["checkout", "--quiet", "main"]);
    dir
}

/// Commits a file `name` on `branch` of the repository in `dir`, and goes back to `main`
pub(super) fn commit_file(dir: &Path, branch: &str, name: &str) {
    git_in(dir, &["checkout", "--quiet", branch]);
    std::fs::write(dir.join(name), "actions: []\n").unwrap();
    git_in(dir, &["add", "."]);
    git_in(dir, &["commit", "--quiet", "-m", name]);
    git_in(dir, &["checkout", "--quiet", "main"]);
}

/// Clones `location` like `resolve`, but into `cache`, and returns the manifest directory
pub(super) fn clone(location: &str, cache: &Path) -> Result<PathBuf, ManifestProviderError> {
    let config = GitConfig::parse(location);
    let clone = cache.join(cache_name(&config));
    fetch_and_clone(&clone, &config)?;
    Ok(config.manifest_dir(clone))
}

/// Updates the clone of `location` in `cache`, like `resolve` does for an existing clone
pub(super) fn update(location: &str, cache: &Path) {
    let config = GitConfig::parse(location);
    update_clone(&cache.join(cache_name(&config)), &config);
}
