mod load;
pub use load::load;
mod providers;
use crate::actions::Actions;
use crate::contexts::{to_rhai, Contexts};
use crate::utilities::password_manager::PasswordManager;
use ::std::hash::Hasher;
use anyhow::{anyhow, Result};
use fnv_rs::{Fnv64, FnvHasher};
pub use providers::register_providers;
pub use providers::ManifestProvider;
use rhai::Engine;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering::SeqCst},
    Arc,
};
use tokio::sync::Barrier;
use tracing::{debug, error, info, instrument};

#[derive(Debug, Clone)]
pub struct DependencyBarrier {
    barrier: Arc<Barrier>,
    can_continue: Arc<AtomicBool>,
}

impl DependencyBarrier {
    pub fn new(n: usize) -> Self {
        Self {
            barrier: Arc::new(Barrier::new(n)),
            can_continue: Arc::new(AtomicBool::new(true)),
        }
    }

    pub async fn wait(&self, result: bool) -> bool {
        let new_can_continue = result & self.can_continue.load(SeqCst);
        self.can_continue.store(new_can_continue, SeqCst);
        self.barrier.wait().await;

        new_can_continue
    }
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    pub barrier: Option<DependencyBarrier>,

    #[serde(skip)]
    pub dependencies: Vec<u64>,
}

impl fmt::Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.name.as_deref().unwrap_or("Cannot extract name")
        )
    }
}

impl AsRef<Manifest> for Manifest {
    fn as_ref(&self) -> &Manifest {
        self
    }
}

impl Manifest {
    pub fn get_name(&self) -> String {
        self.name
            .as_deref()
            .unwrap_or("Cannot extract name")
            .to_string()
    }

    pub fn id(&self) -> u64 {
        let mut hasher = Fnv64::new();
        hasher.update(self.get_name().as_bytes());
        if let Some(root_dir) = &self.root_dir {
            hasher.update(root_dir.to_string_lossy().as_bytes());
        }
        hasher.finish()
    }

    #[instrument(skip_all)]
    pub async fn execute(
        &self,
        dry_run: bool,
        label: Option<String>,
        contexts: &Contexts,
        password_manager: Option<PasswordManager>,
    ) -> Result<()>
where {
        if let Some(label) = label {
            if !self.labels.contains(&label) {
                info!(
                    message = "Skipping manifest, label not found",
                    label = label.as_str()
                );
                return Ok(());
            }
        }

        if let Some(where_condition) = &self.r#where {
            let Ok(result) =
                Engine::new().eval_with_scope::<bool>(&mut to_rhai(contexts), where_condition)
            else {
                return Err(anyhow!(
                    "Unable to evaluate 'where' condition '{where_condition}'"
                ));
            };

            debug!("Result of 'where' condition '{where_condition}' -> '{result}'");
            if !result {
                info!("Skipping manifest, because 'where' conditions were false!");
                return Ok(());
            }
        }

        for action in self.actions.iter() {
            action
                .inner_ref()
                .execute(dry_run, action, self, contexts, password_manager.clone())
                .await?;
        }

        info!("Completed: {self}",);
        Ok(())
    }
}

pub fn resolve(uri: &String) -> Option<PathBuf> {
    let manifest_directory = register_providers()
        .into_iter()
        .filter(|provider| std::ops::Deref::deref(&provider).looks_familiar(uri))
        .fold(None, |path, provider| {
            if path.is_some() {
                return path;
            }

            match provider.resolve(uri.as_str()) {
                Ok(path) => Some(path),
                Err(_) => None,
            }
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
