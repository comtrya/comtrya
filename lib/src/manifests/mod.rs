//! Manifests: finding, loading and naming them.
//!
//! A manifest is a YAML (or TOML) file in the manifest directory. Its name comes from its path,
//! like `dev.git` for `dev/git.yaml`, and other manifests use that name in `depends`:
//!
//! ```yaml
//! where: os.name == "macOS"
//! labels: [work]
//! depends: [dev.git]
//! actions:
//!   - action: file.link
//!     from: zshrc
//!     to: "{{ user.home_dir }}/.zshrc"
//! ```
//!
//! [`resolve`] finds the directory: a local path, or a git repository that a
//! [`ManifestProvider`] clones into the cache. [`load_all`] walks it, skipping `files`
//! directories (where actions find their files) and `Comtrya.yaml`. For each file it renders the
//! templates with the [`contexts`](crate::contexts), then parses it into a [`Manifest`]. A file
//! that fails doesn't stop the others; it ends up in [`Loaded::errors`]:
//!
//! ```
//! use comtrya_lib::contexts::Contexts;
//! use comtrya_lib::manifests::{load_all, LoadErrorKind};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let dir = tempfile::tempdir()?;
//! std::fs::create_dir(dir.path().join("dev"))?;
//! std::fs::write(dir.path().join("dev/git.yaml"), "actions: []")?;
//! std::fs::write(dir.path().join("future.yaml"), "version: 2\nactions: []")?;
//!
//! let loaded = load_all(dir.path().to_path_buf(), &Contexts::default());
//!
//! assert!(loaded.manifests.contains_key("dev.git"));
//! assert_eq!(loaded.errors.len(), 1);
//! assert!(matches!(loaded.errors[0].kind, LoadErrorKind::NewerVersion { found: 2, .. }));
//! # Ok(())
//! # }
//! ```
//!
//! Then the [`executor`](crate::executor) runs the loaded manifests.

/// Reading manifest files
mod load;
pub use load::{
    load, load_all, load_all_ignoring, Ignore, InvalidGlob, LoadError, LoadErrorKind, Loaded,
};
/// Keys nothing reads, warned about while loading
mod unknown_keys;
pub(crate) use unknown_keys::{closest, schema_keys, unknown_keys};
/// Finding the directory with the manifests
mod providers;
use crate::actions::Actions;
use petgraph::prelude::*;
pub use providers::git_cache_dir;
pub use providers::register_providers;
pub use providers::ManifestProvider;
pub use providers::ManifestProviderError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::error;

/// A manifest file: actions, and when and after what they run
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// The manifest format. Only `1` exists so far; comtrya refuses manifests written for a
    /// newer format instead of misreading them
    #[serde(default)]
    pub version: Option<u32>,

    /// A condition on the contexts. The manifest is skipped when it's false
    #[serde(default)]
    pub r#where: Option<String>,

    /// The name, from the file's path below the manifest directory, like `dev.git` for `dev/git.yaml`
    #[serde(default)]
    pub name: Option<String>,

    /// Labels to select manifests with `--label`
    #[serde(default)]
    pub labels: Vec<String>,

    /// Manifests that run before this one, by name
    #[serde(default)]
    pub depends: Vec<String>,

    /// What the manifest does
    #[serde(default)]
    pub actions: Vec<Actions>,

    /// The directory of the manifest file. Its `files` directory holds the files actions use
    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    /// Not used
    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,
}

/// The newest manifest format this version of comtrya understands
pub const MANIFEST_VERSION: u32 = 1;

/// Finds the manifests at `uri`: a local directory, or a git repository that's cloned into the
/// cache. The error is the one of the last provider that tried
///
/// # Errors
///
/// When no provider can resolve `uri`, like a missing directory or a repository that can't be
/// cloned
pub fn resolve(uri: &str) -> Result<PathBuf, ManifestProviderError> {
    let mut error = None;

    for provider in register_providers()
        .iter()
        .filter(|provider| provider.looks_familiar(uri))
    {
        match provider.resolve(uri) {
            Ok(path) => {
                return path
                    .canonicalize()
                    .map_err(|source| ManifestProviderError::NotFound {
                        path: path.display().to_string(),
                        source,
                    })
            }
            Err(err) => error = Some(err),
        }
    }

    Err(error.unwrap_or_else(|| ManifestProviderError::NotFound {
        path: uri.to_string(),
        source: std::io::ErrorKind::NotFound.into(),
    }))
}

/// The name of the manifest at `location`: its path below `manifest_directory`, with `.`
/// between the parts and without `.yaml`/`.yml`/`.toml`. `x/main.yaml` is `x`
///
/// # Errors
///
/// When `location` isn't inside `manifest_directory`
pub fn get_manifest_name(
    manifest_directory: &Path,
    location: &Path,
) -> Result<String, std::path::StripPrefixError> {
    let local_name = location.strip_prefix(manifest_directory)?;
    let manifest_name = local_name
        .components()
        .fold(String::from(""), |mut s, next| {
            if !s.is_empty() {
                s.push('.');
            }

            if let Some(next) = next.as_os_str().to_str() {
                s.push_str(next);
            } else {
                error!("Failed to convert path component to string");
            }

            s
        });

    let manifest_name = manifest_name.trim_end_matches(".yaml");
    let manifest_name = manifest_name.trim_end_matches(".yml");
    let manifest_name = manifest_name.trim_end_matches(".toml");

    Ok(String::from(manifest_name.trim_end_matches(".main")))
}

#[cfg(test)]
#[cfg(unix)]
mod test {
    use super::*;

    #[test]
    fn toml_is_not_part_of_the_name() {
        let manifest_directory = PathBuf::from("/tmp");
        let name = |location: &str| {
            get_manifest_name(&manifest_directory, &PathBuf::from(location)).unwrap()
        };

        assert_eq!("test.git", name("/tmp/test/git.toml"));
        assert_eq!("test", name("/tmp/test/main.toml"));
    }

    #[test]
    fn test_top_level_main_yaml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/main.yaml");

        assert_eq!(
            "main",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_main_yaml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/test/main.yaml");

        assert_eq!(
            "test",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_main_yml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/test/main.yml");

        assert_eq!(
            "test",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_non_main_yaml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/test/hello.yaml");

        assert_eq!(
            "test.hello",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_main_nested_yaml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/test/nested/main.yaml");

        assert_eq!(
            "test.nested",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_non_main_nested_yaml() {
        let manifest_directory = PathBuf::from("/tmp");
        let location = PathBuf::from("/tmp/test/nested/hello.yaml");

        assert_eq!(
            "test.nested.hello",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }
}

#[cfg(test)]
#[cfg(windows)]
mod test {
    use super::*;

    #[test]
    fn test_main_yaml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\main.yaml");

        assert_eq!(
            "test",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_main_yml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\main.yml");

        assert_eq!(
            "test",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_non_main_yaml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\hello.yaml");

        assert_eq!(
            "test.hello",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_main_nested_yaml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\nested\\main.yaml");

        assert_eq!(
            "test.nested",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }

    #[test]
    fn test_non_main_nested_yaml() {
        let manifest_directory = PathBuf::from("C:\\");
        let location = PathBuf::from("C:\\test\\nested\\hello.yaml");

        assert_eq!(
            "test.nested.hello",
            get_manifest_name(&manifest_directory, &location).unwrap()
        );
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn a_local_directory_resolves_to_its_canonical_path() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            dir.path().canonicalize().unwrap(),
            resolve(dir.path().to_str().unwrap()).unwrap()
        );
    }

    #[test]
    fn a_missing_directory_says_which() {
        let error = resolve("/comtrya-no-such-directory").unwrap_err();

        assert!(matches!(error, ManifestProviderError::NotFound { .. }));
        assert!(
            error
                .to_string()
                .starts_with("/comtrya-no-such-directory doesn't exist or can't be read: "),
            "{error}"
        );
    }
}
