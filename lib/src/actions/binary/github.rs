use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tracing::{debug, error};

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Serialize, Deserialize)]
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

            let mut score_terms = vec![
                std::env::consts::OS.to_lowercase(),
                std::env::consts::ARCH.to_lowercase(),
            ];

            let os = os_info::get();
            if os.os_type() == os_info::Type::Macos {
                score_terms.push(String::from("darwin"));
                score_terms.push(String::from("apple"));
            } else {
                score_terms.push(os.os_type().to_string());
            };

            if std::env::consts::ARCH == "aarch64" {
                score_terms.push("arm".to_string());
                score_terms.push("aarch".to_string());
            } else {
                score_terms.push("unknown".to_string());
            };

            if os.bitness() == os_info::Bitness::X32 {
                score_terms.push("32".to_string());
            } else {
                score_terms.push("64".to_string());
            };

            score_terms.iter().for_each(|term| {
                if asset.name.to_lowercase().contains(term.as_str()) {
                    score += 1;
                }
            });

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
            Some(asset) => {
                debug!("Downloading {:?}", asset.url);
                asset
            }
            None => {
                error!("Failed to find a downloadable asset");
                return vec![];
            }
        };

        let steps = vec![
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
        ];

        steps
    }
}
