use crate::actions::Action;
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tracing::debug;

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
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
    fn summarize(&self) -> String {
        format!(
            "Downloading binary from {} to {}",
            self.repository, self.directory
        )
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        // Don't need to do anything if something already exists at the path
        if std::path::Path::new(format!("{}/{}", self.directory, self.name).as_str()).exists() {
            return Ok(vec![]);
        };

        let async_runtime = match Runtime::new() {
            Ok(runtime) => runtime,
            Err(e) => {
                return Err(anyhow!("Failed to create async runtime: {e}"));
            }
        };

        let (owner, repo) = self.repository.split_once('/').ok_or_else(|| {
            anyhow!(
                "Failed to parse repository name: {}",
                self.repository.as_str()
            )
        })?;

        let octocrab = async_runtime.block_on(async { octocrab::instance() });

        let repos = octocrab.repos(owner, repo);
        let releases = repos.releases();

        let result = match &self.version {
            Some(version) if version == "latest" => async_runtime.block_on(releases.get_latest()),
            Some(version) => async_runtime.block_on(releases.get_by_tag(version.as_str())),
            None => async_runtime.block_on(releases.get_latest()),
        };

        let release = match result {
            Ok(release) => release,
            Err(e) => {
                return Err(anyhow!("Failed to find a release: {e}"));
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

            match os.bitness() {
                os_info::Bitness::X32 => score_terms.push("32".to_string()),
                os_info::Bitness::X64 => score_terms.push("64".to_string()),
                _ => (),
            }

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
                            score,
                        })
                    } else {
                        Some(ass)
                    }
                }
                None => Some(GitHubAsset {
                    url: asset.browser_download_url.into(),
                    score,
                }),
            }
        });

        let asset = match asset {
            Some(asset) => {
                debug!("Downloading {:?}", asset.url);
                asset
            }
            None => {
                return Err(anyhow!("Failed to find a downloadable asset"));
            }
        };

        Ok(vec![
            Step {
                atom: Box::new(Download {
                    url: asset.url,
                    to: PathBuf::from(format!("{}/{}", self.directory, self.name)),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Chmod {
                    path: PathBuf::from(format!("{}/{}", self.directory, self.name)),
                    mode: 0o755,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ])
    }
}
