use super::Manifest;
use crate::{
    contexts::{to_tera, Contexts},
    manifests::get_manifest_name,
    tera_functions::register_functions,
};
use ignore::WalkBuilder;
use std::{collections::HashMap, error::Error, fs::canonicalize, ops::Deref, path::PathBuf};
use tera::Tera;
use tracing::{error, span};

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
        .build()
        // Don't walk directories
        .filter(|entry| !entry.clone().unwrap().metadata().unwrap().is_dir())
        .filter(|entry| {
            // There has to be a better way to do this.
            // I couldn't get the TypeBuilder to work
            entry
                .clone()
                .unwrap()
                .file_name()
                .to_str()
                .unwrap()
                .ends_with(".yaml")
                || entry
                    .clone()
                    .unwrap()
                    .file_name()
                    .to_str()
                    .unwrap()
                    .ends_with(".yml")
        })
        // Don't consider anything in a `files` directory a manifest
        .filter(|entry| {
            !entry
                .clone()
                .unwrap()
                .path()
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .eq("files")
        })
        .for_each(|entry| {
            let filename = entry.unwrap();

            let span = span!(
                tracing::Level::INFO,
                "manifest_load",
                manifest = filename.file_name().to_str().unwrap()
            )
            .entered();

            let entry = canonicalize(filename.into_path()).unwrap();
            let contents = std::fs::read_to_string(entry.clone()).unwrap();
            let template = contents.as_str();

            let mut tera = Tera::default();
            register_functions(&mut tera);

            let yaml = match tera.render_str(template, &to_tera(contexts)) {
                Ok(template) => template,
                Err(err) => {
                    match err.source() {
                        Some(err) => error!(message = err.source()),
                        None => error!(message = err.to_string().as_str()),
                    }

                    span.exit();

                    return;
                }
            };

            let mut manifest: Manifest = match serde_yaml::from_str(yaml.deref()) {
                Ok(manifest) => manifest,
                Err(e) => {
                    error!(message = e.to_string().as_str());
                    span.exit();

                    return;
                }
            };

            let name =
                get_manifest_name(&manifest_path, &entry).expect("Failed to get manifest name");

            manifest.root_dir = entry.parent().map(|parent| parent.to_path_buf());

            manifest.name = Some(name.clone());

            manifests.insert(name, manifest);

            span.exit();
        });

    manifests
}
