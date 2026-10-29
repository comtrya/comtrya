//! `binary.github`

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

/// Downloads the release asset that matches this OS and CPU best, unless a file exists at
/// `directory/name` already
#[derive(Clone, Debug, Default, JsonSchema, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryGitHub {
    /// The file name of the binary
    pub name: String,
    /// The directory to put the binary in
    pub directory: String,
    /// The repository, as `owner/repo`
    pub repository: String,
    /// The tag of the release. The latest release without it, or with `latest`
    pub version: Option<String>,
}

/// Where releases are looked up
const GITHUB_API: &str = "https://api.github.com";

/// A release, as the GitHub API returns it
#[derive(Deserialize)]
struct Release {
    /// The files of the release
    assets: Vec<ReleaseAsset>,
}

/// A file of a release
#[derive(Deserialize)]
struct ReleaseAsset {
    /// The file name, like `tool-x86_64-unknown-linux-musl`
    name: String,
    /// Where to download the file
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

impl BinaryGitHub {
    /// Where the binary is installed
    fn path(&self) -> PathBuf {
        PathBuf::from(format!("{}/{}", self.directory, self.name))
    }
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
        if self.path().exists() {
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

        let asset = best_asset(release.assets, &platform_terms())
            .ok_or_else(|| anyhow!("Failed to find a downloadable asset for this OS"))?;
        debug!("Downloading {:?}", asset.browser_download_url);

        Ok(vec![
            Step::new(Download {
                url: asset.browser_download_url,
                to: self.path(),
                sha256: None,
            }),
            Step::new(Chmod {
                path: self.path(),
                mode: 0o755,
            }),
        ])
    }
}

/// Words in asset names that point to this OS and CPU, like `linux`, `x86_64` and `64`
fn platform_terms() -> Vec<String> {
    let os = os_info::get();
    let mut terms = vec![
        std::env::consts::OS.to_lowercase(),
        std::env::consts::ARCH.to_lowercase(),
    ];

    if os.os_type() == os_info::Type::Macos {
        terms.extend([String::from("darwin"), String::from("apple")]);
    } else {
        terms.push(os.os_type().to_string());
    }

    if std::env::consts::ARCH == "aarch64" {
        terms.extend([String::from("arm"), String::from("aarch")]);
    } else {
        // as in `x86_64-unknown-linux-musl`
        terms.push(String::from("unknown"));
    }

    match os.bitness() {
        os_info::Bitness::X32 => terms.push(String::from("32")),
        os_info::Bitness::X64 => terms.push(String::from("64")),
        _ => (),
    }

    terms
}

/// Words in asset names that point to an OS
const OS_TERMS: &[&str] = &[
    "linux", "darwin", "apple", "macos", "windows", "freebsd", "netbsd", "openbsd",
];

/// Whether `name` names an OS, but none of the OSes in `terms`, like `tool-linux` on Windows
fn for_another_os(name: &str, terms: &[String]) -> bool {
    let (ours, others): (Vec<&str>, Vec<&str>) = OS_TERMS
        .iter()
        .filter(|os| name.contains(**os))
        .partition(|os| terms.iter().any(|term| term == *os));
    ours.is_empty() && !others.is_empty()
}

/// The asset whose name contains the most `terms`, leaving out assets for another OS. The first
/// one wins a tie
fn best_asset(assets: Vec<ReleaseAsset>, terms: &[String]) -> Option<ReleaseAsset> {
    let score = |asset: &ReleaseAsset| {
        let name = asset.name.to_lowercase();
        terms
            .iter()
            .filter(|term| name.contains(term.as_str()))
            .count()
    };

    let mut best: Option<(usize, ReleaseAsset)> = None;
    for asset in assets {
        if for_another_os(&asset.name.to_lowercase(), terms) {
            continue;
        }

        let asset_score = score(&asset);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| asset_score > *best_score)
        {
            best = Some((asset_score, asset));
        }
    }

    best.map(|(_, asset)| asset)
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

    fn assets(names: &[&str]) -> Vec<ReleaseAsset> {
        names
            .iter()
            .map(|name| ReleaseAsset {
                name: name.to_string(),
                browser_download_url: format!("https://example.com/{name}"),
            })
            .collect()
    }

    fn terms(terms: &[&str]) -> Vec<String> {
        terms.iter().map(|term| term.to_string()).collect()
    }

    #[test]
    fn the_asset_with_the_most_terms_wins() {
        let assets = assets(&["tool-windows.zip", "tool-x86_64-linux", "tool-linux"]);

        let best = best_asset(assets, &terms(&["linux", "x86_64"])).unwrap();

        assert_eq!("tool-x86_64-linux", best.name);
    }

    #[test]
    fn the_first_asset_wins_a_tie() {
        let assets = assets(&["tool-Linux.tar.gz", "tool-linux.zip"]);

        let best = best_asset(assets, &terms(&["linux"])).unwrap();

        assert_eq!("tool-Linux.tar.gz", best.name);
    }

    #[test]
    fn an_asset_is_picked_even_without_a_match() {
        let best = best_asset(assets(&["tool.zip"]), &terms(&["linux"])).unwrap();

        assert_eq!("tool.zip", best.name);
        assert!(best_asset(vec![], &terms(&["linux"])).is_none());
    }

    #[test]
    fn an_asset_for_another_os_is_never_picked() {
        let windows = terms(&["windows", "x86_64", "unknown", "64"]);
        let release = assets(&[
            "tool-x86_64-unknown-linux-musl",
            "tool-x86_64-pc-windows-msvc.exe",
        ]);

        let best = best_asset(release, &windows).unwrap();

        assert_eq!("tool-x86_64-pc-windows-msvc.exe", best.name);
        assert!(best_asset(assets(&["tool-linux"]), &windows).is_none());
    }
}
