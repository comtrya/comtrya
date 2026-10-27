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
use tracing::debug;

#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryGitHub {
    pub name: String,
    pub directory: String,
    pub repository: String,
    pub version: Option<String>,
}

const GITHUB_API: &str = "https://api.github.com";

#[derive(Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

/// A release from the GitHub API: the latest one, or the one tagged `version`. Uses
/// `GITHUB_TOKEN` when it's set, which raises the API's rate limit
fn fetch_release(
    api: &str,
    owner: &str,
    repo: &str,
    version: Option<&str>,
) -> anyhow::Result<Release> {
    let url = match version {
        None | Some("latest") => format!("{api}/repos/{owner}/{repo}/releases/latest"),
        Some(tag) => format!("{api}/repos/{owner}/{repo}/releases/tags/{tag}"),
    };

    let mut request = reqwest::blocking::Client::new()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", concat!("comtrya/", env!("CARGO_PKG_VERSION")));
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        request = request.bearer_auth(token);
    }

    Ok(request.send()?.error_for_status()?.json()?)
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

        let (owner, repo) = self.repository.split_once('/').ok_or_else(|| {
            anyhow!(
                "Failed to parse repository name: {}",
                self.repository.as_str()
            )
        })?;

        let release = fetch_release(GITHUB_API, owner, repo, self.version.as_deref())
            .map_err(|e| anyhow!("Failed to find a release: {e}"))?;

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
                            url: asset.browser_download_url.clone(),
                            score,
                        })
                    } else {
                        Some(ass)
                    }
                }
                None => Some(GitHubAsset {
                    url: asset.browser_download_url.clone(),
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
                    sha256: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Answers one request with `body`, and sends back the request line
    fn serve(status: u16, body: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let api = format!("http://{}", listener.local_addr().unwrap());
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]).to_string();
            sender.send(request).unwrap();
            let header = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body.as_bytes());
        });
        (api, receiver)
    }

    const RELEASE: &str = r#"{"tag_name":"v1.0.0","assets":[{"name":"tool-x86_64-unknown-linux-musl","browser_download_url":"https://example.com/tool","size":1}]}"#;

    #[test]
    fn fetches_the_latest_release() {
        let (api, request) = serve(200, RELEASE);

        let release = fetch_release(&api, "comtrya", "comtrya", None).unwrap();

        assert_eq!(1, release.assets.len());
        assert_eq!("tool-x86_64-unknown-linux-musl", release.assets[0].name);
        assert_eq!(
            "https://example.com/tool",
            release.assets[0].browser_download_url
        );
        let request = request.recv().unwrap();
        assert!(
            request.starts_with("GET /repos/comtrya/comtrya/releases/latest "),
            "{request}"
        );
        assert!(
            request.to_lowercase().contains("user-agent: comtrya/"),
            "{request}"
        );
    }

    #[test]
    fn fetches_a_release_by_tag() {
        let (api, request) = serve(200, RELEASE);

        fetch_release(&api, "comtrya", "comtrya", Some("v0.9.2")).unwrap();

        let request = request.recv().unwrap();
        assert!(
            request.starts_with("GET /repos/comtrya/comtrya/releases/tags/v0.9.2 "),
            "{request}"
        );
    }

    #[test]
    fn a_missing_release_is_an_error() {
        let (api, _request) = serve(404, r#"{"message":"Not Found"}"#);

        let error = fetch_release(&api, "comtrya", "comtrya", Some("v0.0.0"))
            .err()
            .unwrap();

        assert!(error.to_string().contains("404"), "{error}");
    }
}
