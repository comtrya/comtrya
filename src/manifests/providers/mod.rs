mod git;
use git::GitManifestProvider;
mod local;
use local::LocalManifestProvider;
use std::path::PathBuf;

pub fn register_providers() -> Vec<Box<dyn ManifestProvider>> {
    vec![
        Box::new(GitManifestProvider),
        Box::new(LocalManifestProvider),
    ]
}

#[derive(Debug, PartialEq)]
pub enum ManifestProviderError {
    NoResolution,
}

/// ManifestProviders are responsible for taking a String
/// and returning a `PathBuf`. Providers, such as Git, are
/// responsible for accepting the String and cloning the
/// repository, in-order to return the `PathBuf`.
pub trait ManifestProvider {
    /// This functions is called to establish if it could potentially
    /// be able to resolve the url provided
    fn looks_familiar(&self, url: &String) -> bool;

    /// This function is responsible for returning a PathBuf with
    /// the directory containing the manifests
    fn resolve(&self, url: &String) -> Result<PathBuf, ManifestProviderError>;
}
