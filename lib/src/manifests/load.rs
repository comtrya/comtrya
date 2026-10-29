//! Reads manifest files: renders their templates, checks their version, and parses them

use super::{Manifest, MANIFEST_VERSION};
use crate::{
    contexts::{evaluate_condition, to_tera, Contexts},
    manifests::get_manifest_name,
    tera_functions::register_functions,
};
use ignore::WalkBuilder;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::HashMap,
    error::Error,
    ffi::OsStr,
    fs::canonicalize,
    path::{Path, PathBuf},
};
use tera::Tera;
use thiserror::Error as ThisError;
use tracing::{debug, error, span, warn};

/// Manifests that loaded, and the ones that didn't
#[derive(Debug, Default)]
pub struct Loaded {
    /// The manifests by name
    pub manifests: HashMap<String, Manifest>,
    /// The files that failed
    pub errors: Vec<LoadError>,
}

/// A manifest file that couldn't be loaded
#[derive(Debug, ThisError)]
#[error("Manifest '{name}' ({}) can't be loaded: {kind}", path.display())]
pub struct LoadError {
    /// The manifest's name, as other manifests refer to it in `depends`
    pub name: String,
    /// The file
    pub path: PathBuf,
    /// What went wrong
    pub kind: LoadErrorKind,
}

/// Why a manifest file can't be loaded
#[derive(Debug, ThisError)]
pub enum LoadErrorKind {
    /// The file can't be read
    #[error("the file can't be read: {0}")]
    Read(#[source] std::io::Error),

    /// A template in the file fails
    #[error("templates can't be rendered: {0}")]
    Render(String),

    /// The file isn't a valid manifest
    #[error("{0}")]
    Parse(String),

    /// The file needs a newer comtrya
    #[error("it uses manifest version {found}, but this comtrya only supports version {supported}. Update comtrya")]
    NewerVersion {
        /// The `version` of the file
        found: u32,
        /// The newest version this comtrya supports
        supported: u32,
    },
}

/// The part of a manifest that is readable before its templates are rendered
#[derive(Deserialize)]
struct Unrendered {
    /// The manifest format
    #[serde(default)]
    version: Option<u32>,

    /// The condition, to skip a manifest whose templates fail where it doesn't apply
    #[serde(default)]
    r#where: Option<String>,
}

/// Loads all manifests below `manifest_path`, and logs the ones that can't be loaded
pub fn load(manifest_path: PathBuf, contexts: &Contexts) -> HashMap<String, Manifest> {
    let loaded = load_all(manifest_path, contexts);
    for err in &loaded.errors {
        error!("{err}");
    }
    loaded.manifests
}

/// Loads all manifests below `manifest_path`, and returns the ones that can't be loaded as errors
pub fn load_all(manifest_path: PathBuf, contexts: &Contexts) -> Loaded {
    let mut loaded = Loaded::default();

    let mut walker = WalkBuilder::new(&manifest_path);

    // FIXME: get rid of all .unwrap() calls
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .filter_entry({
            // The config file next to the manifests isn't a manifest
            let config = manifest_path.join("Comtrya.yaml");
            move |entry| {
                let files_dir = entry.file_type().is_some_and(|ft| ft.is_dir())
                    && entry.file_name() == OsStr::new("files");
                !files_dir && entry.path() != config
            }
        })
        .build()
        // Don't walk directories
        .filter(|entry| {
            !entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.metadata().ok().map(|entry| entry.is_dir()))
                .unwrap_or(false)
        })
        .filter(|entry| {
            entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.file_name().to_str())
                .map(|file_name| {
                    file_name.ends_with(".yaml")
                        || file_name.ends_with(".yml")
                        || file_name.ends_with(".toml")
                })
                .unwrap_or(false)
        })
        .flatten()
        .for_each(|entry| {
            let span = span!(
                tracing::Level::INFO,
                "manifest_load",
                manifest = entry.file_name().to_str()
            )
            .entered();

            let path = canonicalize(entry.path()).unwrap_or_else(|_| entry.into_path());
            let name = get_manifest_name(&manifest_path, &path)
                .unwrap_or_else(|_| path.display().to_string());

            match load_file(&path, contexts) {
                Ok(mut manifest) => {
                    manifest.root_dir = path.parent().map(|parent| parent.to_path_buf());
                    manifest.name = Some(name.clone());
                    loaded.manifests.insert(name, manifest);
                }
                Err(kind) => loaded.errors.push(LoadError { name, path, kind }),
            }

            span.exit();
        });

    loaded
}

/// Reads, renders and parses one manifest file
fn load_file(path: &Path, contexts: &Contexts) -> Result<Manifest, LoadErrorKind> {
    let template = std::fs::read_to_string(path).map_err(LoadErrorKind::Read)?;

    // Checked before rendering, a newer format may use templates differently
    let version = parse::<Unrendered>(&template, path)
        .ok()
        .and_then(|unrendered| unrendered.version);
    check_version(version)?;

    let mut tera = Tera::default();
    register_functions(&mut tera);

    let rendered = match tera.render_str(&template, &to_tera(contexts)) {
        Ok(rendered) => rendered,
        // A template that only works where the manifest applies, like an env var
        // that only exists on Windows, is fine when `where` skips it anyway (#283)
        Err(err) => {
            return match false_where(&template, path, contexts) {
                Some(condition) => {
                    debug!("Not rendering templates, because the 'where' condition is false");
                    Ok(Manifest {
                        r#where: Some(condition),
                        ..Default::default()
                    })
                }
                None => Err(LoadErrorKind::Render(error_chain(&err))),
            }
        }
    };

    let unknown = unknown_keys(&rendered, path);
    let manifest = parse::<Manifest>(&rendered, path).map_err(|err| {
        // A misspelled required field is the likely cause
        let hints: String = unknown.iter().map(|key| format!("; {key}")).collect();
        LoadErrorKind::Parse(format!("{err:#}{hints}"))
    })?;
    check_version(manifest.version)?;
    for key in unknown {
        warn!("{}: {key}", path.display());
    }

    Ok(manifest)
}

/// The keys of the manifest nothing reads, like a misspelled field
fn unknown_keys(rendered: &str, path: &Path) -> Vec<super::UnknownKey> {
    parse::<serde_json::Value>(rendered, path)
        .map(|manifest| super::unknown_keys(&manifest))
        .unwrap_or_default()
}

/// Tera puts the useful part of a message into the error's sources
fn error_chain(err: &dyn Error) -> String {
    let mut message = err.to_string();
    let mut source = err.source();
    while let Some(err) = source {
        message.push_str(": ");
        message.push_str(&err.to_string());
        source = err.source();
    }
    message
}

/// Parses YAML or TOML, by the file extension
fn parse<T: DeserializeOwned>(contents: &str, path: &Path) -> anyhow::Result<T> {
    match path.extension().and_then(OsStr::to_str) {
        Some("yaml") | Some("yml") => Ok(serde_yaml_ng::from_str(contents)?),
        Some("toml") => Ok(toml::from_str(contents)?),
        _ => Err(anyhow::anyhow!("Unrecognized file extension for manifest")),
    }
}

/// Fails for a manifest format newer than this comtrya knows
fn check_version(version: Option<u32>) -> Result<(), LoadErrorKind> {
    match version {
        Some(found) if found > MANIFEST_VERSION => Err(LoadErrorKind::NewerVersion {
            found,
            supported: MANIFEST_VERSION,
        }),
        _ => Ok(()),
    }
}

/// The manifest's `where` condition if it is false, read without rendering templates
fn false_where(contents: &str, path: &Path, contexts: &Contexts) -> Option<String> {
    let condition = parse::<Unrendered>(contents, path).ok()?.r#where?;

    evaluate_condition(&condition, contexts)
        .is_ok_and(|result| !result)
        .then_some(condition)
}
