mod local;
use local::LocalManifestProvider;
use std::path::PathBuf;
mod git;
pub use git::git_cache_dir;
use git::GitManifestProvider;

pub fn register_providers() -> Vec<Box<dyn ManifestProvider>> {
    vec![
        Box::new(LocalManifestProvider),
        Box::new(GitManifestProvider),
    ]
}

/// Why the manifests at a location can't be found
#[derive(Debug, thiserror::Error)]
pub enum ManifestProviderError {
    #[error("{path} doesn't exist or can't be read: {source}")]
    NotFound {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("there is no cache directory to clone git manifests into")]
    NoCacheDir,

    #[error("can't create the cache directory {}: {source}", path.display())]
    CacheDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{url} is not a valid git URL: {message}")]
    InvalidUrl { url: String, message: String },

    #[error("can't clone {url}: {message}")]
    Clone { url: String, message: String },
}

/// ManifestProviders are responsible for taking a String
/// and returning a `PathBuf`. Providers, such as Git, are
/// responsible for accepting the String and cloning the
/// repository, in-order to return the `PathBuf`.
pub trait ManifestProvider {
    /// This functions is called to establish if it could potentially
    /// be able to resolve the url provided
    fn looks_familiar(&self, url: &str) -> bool;

    /// This function is responsible for returning a PathBuf with
    /// the directory containing the manifests
    fn resolve(&self, url: &str) -> Result<PathBuf, ManifestProviderError>;
}
