mod load;
pub use load::load;
mod providers;
use crate::actions::Actions;
use petgraph::prelude::*;
pub use providers::register_providers;
pub use providers::ManifestProvider;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::error;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub r#where: Option<String>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub labels: Vec<String>,

    #[serde(default)]
    pub depends: Vec<String>,

    #[serde(default)]
    pub actions: Vec<Actions>,

    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,
}

pub fn resolve(uri: &String) -> Option<PathBuf> {
    let manifest_directory = register_providers()
        .into_iter()
        .filter(|provider| std::ops::Deref::deref(&provider).looks_familiar(uri))
        .fold(None, |path, provider| {
            if path.is_some() {
                return path;
            }

            provider.resolve(uri.as_str()).ok()
        });

    let manifest_directory = match manifest_directory {
        Some(dir) => dir.canonicalize().expect("Failed to canonicalize path"),
        None => {
            error!("Failed to find manifests at {}", &uri);
            panic!();
        }
    };

    Some(manifest_directory)
}

pub fn get_manifest_name(manifest_directory: &Path, location: &Path) -> anyhow::Result<String> {
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

    Ok(String::from(manifest_name.trim_end_matches(".main")))
}

#[cfg(test)]
#[cfg(unix)]
mod test {
    use super::*;

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
