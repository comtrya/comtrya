//! Downloads a file

use crate::atoms::{Check, Error, Result};
use crate::runtime;
use serde::{Deserialize, Serialize};

use super::super::Change;
use crate::utilities::sha256 as digest;
use std::io::Write;
use std::{fs::File, path::PathBuf};

/// Downloads a file, unless it exists and matches `sha256`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Download {
    /// What to download
    pub url: String,
    /// Where to save it
    pub to: PathBuf,
    /// Expected SHA-256 of the file as lowercase hex. Without it, an existing file is kept
    pub sha256: Option<String>,
}

impl std::fmt::Display for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpDownload from {} to {}", self.url, self.to.display())
    }
}

impl Change for Download {
    fn check(&self) -> Result<Check> {
        let differs = match (&self.sha256, std::fs::read(&self.to)) {
            (_, Err(_)) => true,
            // Download again if the file doesn't match, e.g. after the URL and hash were updated
            (Some(expected), Ok(existing)) => digest(existing.as_slice()) != *expected,
            (None, Ok(_)) => false,
        };

        Ok(Check::drift_if(differs))
    }

    fn execute(&self) -> Result<()> {
        let content = runtime::block_on(fetch(&self.url)).map_err(|source| Error::Download {
            url: self.url.clone(),
            source,
        })?;

        // Check before writing, so a bad download never replaces the existing file
        if let Some(expected) = &self.sha256 {
            let actual = digest(&content);
            if actual != *expected {
                return Err(Error::Checksum {
                    url: self.url.clone(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }

        File::create(&self.to)
            .and_then(|mut file| file.write_all(&content))
            .map_err(Error::io("write", &self.to))?;

        Ok(())
    }
}

/// The body of `url`, or an error if the server answers with one
async fn fetch(url: &str) -> reqwest::Result<Vec<u8>> {
    let response = reqwest::get(url).await?.error_for_status()?;
    Ok(response.bytes().await?.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    /// Serves `body` with `status` on localhost for every request, returns the URL
    fn serve(status: u16, body: &'static [u8]) -> String {
        use std::io::Read;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut request = [0; 1024];
                let _ = stream.read(&mut request);
                let header = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body);
            }
        });
        url
    }

    const BODY: &[u8] = b"hello comtrya\n";

    fn atom(url: String, to: PathBuf, sha256: Option<&str>) -> Download {
        Download {
            url,
            to,
            sha256: sha256.map(String::from),
        }
    }

    #[test]
    fn writes_the_file_when_the_checksum_matches() {
        let dir = tempdir().unwrap();
        let to = dir.path().join("file");
        let atom = atom(serve(200, BODY), to.clone(), Some(&digest(BODY)));

        assert_eq!(Check::Drift, atom.check().unwrap());
        atom.execute().unwrap();

        assert_eq!(BODY, std::fs::read(&to).unwrap().as_slice());
        assert_eq!(Check::InSync, atom.check().unwrap());
    }

    #[test]
    fn a_mismatching_checksum_fails_and_keeps_the_existing_file() {
        let dir = tempdir().unwrap();
        let to = dir.path().join("file");
        std::fs::write(&to, "old").unwrap();
        let expected = digest("something else");
        let atom = atom(serve(200, BODY), to.clone(), Some(&expected));

        let err = atom.execute().unwrap_err().to_string();

        assert!(
            err.contains(&expected) && err.contains(&digest(BODY)),
            "{err}"
        );
        assert_eq!("old", std::fs::read_to_string(&to).unwrap());
    }

    #[test]
    fn an_existing_file_with_another_checksum_is_downloaded_again() {
        let dir = tempdir().unwrap();
        let to = dir.path().join("file");
        std::fs::write(&to, "outdated").unwrap();
        let atom = atom(serve(200, BODY), to.clone(), Some(&digest(BODY)));

        assert_eq!(Check::Drift, atom.check().unwrap());
        atom.execute().unwrap();

        assert_eq!(BODY, std::fs::read(&to).unwrap().as_slice());
    }

    #[test]
    fn without_checksum_an_existing_file_is_kept() {
        let dir = tempdir().unwrap();
        let to = dir.path().join("file");
        std::fs::write(&to, "anything").unwrap();

        assert_eq!(
            Check::InSync,
            atom(serve(200, BODY), to, None).check().unwrap()
        );
    }

    #[test]
    fn http_errors_fail_instead_of_saving_the_error_page() {
        let dir = tempdir().unwrap();
        let to = dir.path().join("file");
        let atom = atom(serve(404, b"not found"), to.clone(), None);

        let err = format!("{:#}", anyhow::Error::from(atom.execute().unwrap_err()));

        assert!(err.starts_with("can't download http://"), "{err}");
        assert!(err.contains("404"), "{err}");
        assert!(!to.exists());
    }

    #[test]
    fn it_can() {
        let tmpdir = tempdir().unwrap();
        let to_file = tmpdir.path().join("download");

        let atom = Download {
            url: String::from("https://www.google.com/images/branding/googlelogo/2x/googlelogo_color_272x92dp.png"),
            to: to_file,
            sha256: None,
        };

        assert_eq!(Check::Drift, atom.check().unwrap());

        let result = atom.execute();
        assert!(result.is_ok());
        assert_eq!(Check::InSync, atom.check().unwrap());
    }
}
