use super::{ManifestProvider, ManifestProviderError};

use std::path::PathBuf;

#[derive(Debug)]
pub struct LocalManifestProvider;

impl ManifestProvider for LocalManifestProvider {
    /// The Local provider is essentially our final chance to
    /// resolve the url. It'll match anything and try to find a
    /// directory
    fn looks_familiar(&self, _url: &String) -> bool {
        true
    }

    fn resolve(&self, url: &String) -> Result<PathBuf, ManifestProviderError> {
        if url.starts_with("/") {
            return self.resolve_absolute_url(url);
        }

        self.resolve_relative_url(url)
    }
}

impl LocalManifestProvider {
    fn resolve_absolute_url(&self, url: &String) -> Result<PathBuf, ManifestProviderError> {
        PathBuf::from(url)
            .canonicalize()
            .map_err(|_| ManifestProviderError::NoResolution)
    }

    fn resolve_relative_url(&self, url: &String) -> Result<PathBuf, ManifestProviderError> {
        std::env::current_dir()
            .unwrap()
            .join(url)
            .canonicalize()
            .map_err(|_| ManifestProviderError::NoResolution)
    }
}

#[cfg(test)]
mod test {
    use super::super::ManifestProviderError;
    use super::LocalManifestProvider;

    #[test]
    fn test_resolve_absolute_url() {
        let local_manifest_provider = LocalManifestProvider {};

        let cwd = std::env::current_dir().unwrap();
        let cwd_string = String::from(cwd.to_str().unwrap());

        assert_eq!(
            cwd,
            local_manifest_provider
                .resolve_absolute_url(&cwd_string)
                .unwrap()
        );

        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            local_manifest_provider.resolve_absolute_url(&String::from("/never-resolve"))
        );
    }

    #[test]
    fn test_resolve_relative_url() {
        let local_manifest_provider = LocalManifestProvider {};

        let cwd = std::env::current_dir().unwrap().join("examples");

        assert_eq!(
            cwd,
            local_manifest_provider
                .resolve_relative_url(&String::from("./examples"))
                .unwrap()
        );

        assert_eq!(
            Err(ManifestProviderError::NoResolution),
            local_manifest_provider.resolve_relative_url(&String::from("never-resolve"))
        );
    }
}
