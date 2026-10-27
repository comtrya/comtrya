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
use tracing::{debug, error, span};

/// The part of a manifest that is readable before its templates are rendered
#[derive(Deserialize)]
struct Unrendered {
    #[serde(default)]
    version: Option<u32>,

    #[serde(default)]
    r#where: Option<String>,
}

pub fn load(manifest_path: PathBuf, contexts: &Contexts) -> HashMap<String, Manifest> {
    let mut manifests: HashMap<String, Manifest> = HashMap::new();

    let mut walker = WalkBuilder::new(&manifest_path);

    // FIXME: get rid of all .unwrap() calls
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .filter_entry(|entry| {
            !(entry.file_type().is_some_and(|ft| ft.is_dir())
            && entry.file_name() == OsStr::new("files"))
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
        .for_each(|entry| {
            if let Ok(filename) = entry {
                let span = span!(
                    tracing::Level::INFO,
                    "manifest_load",
                    manifest = filename.file_name().to_str()
                )
                .entered();

                let entry = canonicalize(filename.into_path()).ok().unwrap_or_default();
                let contents =
                    std::fs::read_to_string(entry.clone()).unwrap_or_else(|_| String::from(""));
                let template = contents.as_str();

                // Checked before rendering, a newer format may use templates differently
                let version = parse::<Unrendered>(template, &entry)
                    .ok()
                    .and_then(|unrendered| unrendered.version);
                if let Err(err) = check_version(version) {
                    error!("Manifest in file with path '{}' is skipped. Reason: {err}", &entry.display());
                    span.exit();
                    return;
                }

                let mut tera = Tera::default();
                register_functions(&mut tera);

                let manifest = match tera.render_str(template, &to_tera(contexts)) {
                    Ok(rendered) => parse::<Manifest>(&rendered, &entry),
                    // A template that only works where the manifest applies, like an env var
                    // that only exists on Windows, is fine when `where` skips it anyway (#283)
                    Err(err) => match false_where(template, &entry, contexts) {
                        Some(condition) => {
                            debug!("Not rendering templates, because the 'where' condition is false");
                            Ok(Manifest {
                                r#where: Some(condition),
                                ..Default::default()
                            })
                        }
                        None => {
                            match err.source() {
                                Some(err) => error!(message = err.source()),
                                None => error!(message = err.to_string().as_str()),
                            }

                            span.exit();

                            return;
                        }
                    },
                };

                match manifest.and_then(|manifest| check_version(manifest.version).map(|_| manifest)) {
                    Ok(mut manifest) => {
                        let name = get_manifest_name(&manifest_path, &entry)
                            .expect("Failed to get manifest name");

                        manifest.root_dir = entry.parent().map(|parent| parent.to_path_buf());

                        manifest.name = Some(name.clone());

                        manifests.insert(name, manifest);
                    }
                    Err(err) => {
                        let manifest_name =
                            get_manifest_name(&manifest_path, &entry).unwrap_or_default();

                        error!("Manifest '{manifest_name}' in file with path '{}' cannot be parsed. Reason: {err}", &entry.display());
                    }
                }

                span.exit();
            }
        });

    manifests
}

fn parse<T: DeserializeOwned>(contents: &str, path: &Path) -> anyhow::Result<T> {
    match path.extension().and_then(OsStr::to_str) {
        Some("yaml") | Some("yml") => Ok(serde_yaml_ng::from_str(contents)?),
        Some("toml") => Ok(toml::from_str(contents)?),
        _ => Err(anyhow::anyhow!("Unrecognized file extension for manifest")),
    }
}

fn check_version(version: Option<u32>) -> anyhow::Result<()> {
    match version {
        Some(version) if version > MANIFEST_VERSION => Err(anyhow::anyhow!(
            "it uses manifest version {version}, but this comtrya only supports version {MANIFEST_VERSION}. Update comtrya"
        )),
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
