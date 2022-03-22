use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
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

            let os = os_info::get();
            let os_type = if os.os_type() == os_info::Type::Macos {
                "darwin".to_string()
            } else {
                os.os_type().to_string()
            };

            let simple_bitness = if os.bitness() == os_info::Bitness::X32 {
                "32".to_string()
            } else {
                "64".to_string()
            };

            let simple_aarch = if std::env::consts::ARCH == "aarch64" {
                "arm".to_string()
            } else {
                "unknown".to_string()
            };

            vec![
                &std::env::consts::OS.to_lowercase(),
                &os_type,
                &std::env::consts::ARCH.to_lowercase(),
                &simple_bitness,
                &simple_aarch,
            ]
            .iter()
            .for_each(|term| {
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
            Some(asset) => asset,
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
