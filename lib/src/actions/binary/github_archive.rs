use crate::actions::Action;
use crate::atoms::file::Unarchive;
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::warn;

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveGithub {
    repository: String,
    version: String,
    extension: String,
    dest: PathBuf,
    path: PathBuf,
}

impl Action for ArchiveGithub {
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        if self.dest.to_str().unwrap().is_empty()
            || self.repository.is_empty()
            || self.version.is_empty()
            || self.extension.is_empty()
            || self.path.to_str().unwrap().is_empty()
        {
            warn!(message = "Can not download an archive with information missing");
        }

        let url: String = "https://github.com/".to_owned()
            + self.repository.as_str()
            + "/archive/refs/tags/"
            + self.version.as_str()
            + "."
            + self.extension.as_str();

        println!("{}", url);

        let step: Vec<Step> = vec![
            Step {
                atom: Box::new(Download {
                    url: url.clone(),
                    to: self.path.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Unarchive {
                    origin: self.path.clone(),
                    dest: self.dest.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ];

        Ok(step)
    }
}
