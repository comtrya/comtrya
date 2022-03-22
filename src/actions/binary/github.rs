use std::path::PathBuf;

use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use tracing::error;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BinaryGitHub {
    pub name: String,
    pub directory: String,
    pub repository: String,
    pub version: Option<String>,
}

struct GitHubAsset {
    pub url: String,
    pub score: i32,
}

impl Action for BinaryGitHub {
    fn plan(&self, _: &Manifest, _: &Contexts) -> Vec<Step> {
        let async_runtime = match Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                error!("Failed to create async runtime: {}", e);
                return vec![];
            }
        };

        let (owner, repo) = self.repository.split_once("/").unwrap();

        let octocrab = octocrab::instance();

        let repos = octocrab.repos(owner, repo);
        let releases = repos.releases();

        let result = match &self.version {
            Some(version) => async_runtime.block_on(releases.get_by_tag(version.as_str())),
            None => async_runtime.block_on(releases.get_latest()),
        };

        let release = match result {
            Ok(release) => release,
            Err(e) => {
                error!("Failed to find a release: {}", e);
                return vec![];
            }
        };

        let asset: Option<GitHubAsset> = release.assets.into_iter().fold(None, |acc, asset| {
            let mut score = 0;

            if asset
                .name
                .to_lowercase()
                .contains(&std::env::consts::OS.to_lowercase())
            {
                score = score + 10;
            }

            if asset
                .name
                .to_lowercase()
                .contains(&std::env::consts::ARCH.to_lowercase())
            {
                score = score + 20;
            }

            match acc {
                Some(ass) => {
                    if score > ass.score {
                        Some(GitHubAsset {
                            url: asset.browser_download_url.into(),
                            score: score,
                        })
                    } else {
                        Some(ass)
                    }
                }
                None => Some(GitHubAsset {
                    url: asset.browser_download_url.into(),
                    score: score,
                }),
            }
        });

        let asset = match asset {
            Some(asset) => asset,
            None => {
                error!("Failed to find a downloadable asset");
                return vec![];
            }
        };

        vec![
            Step {
                atom: Box::new(Download {
                    url: asset.url,
                    to: PathBuf::from(format!("{}/{}", &self.directory, &self.name)),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Chmod {
                    path: PathBuf::from(format!("{}/{}", &self.directory, &self.name)),
                    mode: 0o755,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ]
    }
}
