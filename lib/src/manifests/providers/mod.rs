//! Manifest providers: where `manifest_paths` point to, local directories or git repositories

/// Local directories
mod local;
use local::LocalManifestProvider;
use std::path::PathBuf;
/// Git repositories, cloned into the cache
mod git;
pub use git::git_cache_dir;
use git::GitManifestProvider;

/// The providers, in the order they are tried
pub fn register_providers() -> Vec<Box<dyn ManifestProvider>> {
    vec![
        Box::new(LocalManifestProvider),
        Box::new(GitManifestProvider),
    ]
}

/// Why the manifests at a location can't be found
#[derive(Debug, thiserror::Error)]
pub enum ManifestProviderError {
    /// The location doesn't exist
    #[error("{path} doesn't exist or can't be read: {source}")]
    NotFound {
        /// The location
        path: String,
        /// Why it can't be read
        #[source]
        source: std::io::Error,
    },

    /// The system has no cache directory
    #[error("there is no cache directory to clone git manifests into")]
    NoCacheDir,

    /// The cache directory can't be created
    #[error("can't create the cache directory {}: {source}", path.display())]
    CacheDir {
        /// The directory
        path: PathBuf,
        /// Why it can't be created
        #[source]
        source: std::io::Error,
    },

    /// The git URL is invalid
    #[error("{url} is not a valid git URL: {message}")]
    InvalidUrl {
        /// The URL
        url: String,
        /// What's wrong with it
        message: String,
    },

    /// The repository can't be cloned
    #[error("can't clone {url}: {message}")]
    Clone {
        /// The URL
        url: String,
        /// Why the clone failed
        message: String,
    },
}

/// Turns an entry of `manifest_paths` into a local directory, cloning a git repository first
/// if needed
pub trait ManifestProvider {
    /// Whether this provider may be able to resolve `url`
    fn looks_familiar(&self, url: &str) -> bool;

    /// The directory with the manifests
    fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError>;
}
