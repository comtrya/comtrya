//! The HTTP client of the update check, on comtrya's own reqwest and tokio runtime. The default
//! client of `update_informer` is ureq, which would build a second TLS stack

use comtrya_lib::runtime;
use serde::de::DeserializeOwned;
use std::time::Duration;
use update_informer::http_client::{HeaderMap, HttpClient};

/// crates.io refuses requests without a User-Agent. It names comtrya, nothing about the user
const USER_AGENT: &str = concat!(
    "comtrya/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/comtrya/comtrya)"
);

/// Asks the registry for the latest version, for [`update_informer`]. A failure, or no answer
/// within the timeout, only means no update hint
pub struct UpdateClient;

impl HttpClient for UpdateClient {
    fn get<T: DeserializeOwned>(
        url: &str,
        timeout: Duration,
        headers: HeaderMap,
    ) -> update_informer::Result<T> {
        comtrya_lib::tls::install_crypto_provider();
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .build()?;
        let request = headers
            .into_iter()
            .fold(client.get(url), |request, (key, value)| {
                request.header(key, value)
            });

        Ok(runtime::block_on(async {
            request.send().await?.error_for_status()?.json().await
        })?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// Answers one request with `body` as JSON, and returns the request's head
    fn serve_once(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/versions", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut head = String::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            head
        });
        (url, server)
    }

    #[test]
    fn it_reads_the_answer_and_names_comtrya() {
        let (url, server) = serve_once(r#"{"versions":[{"num":"9.9.9"}]}"#);

        let answer: serde_json::Value =
            UpdateClient::get(&url, Duration::from_secs(5), HeaderMap::default()).unwrap();

        assert_eq!("9.9.9", answer["versions"][0]["num"]);
        let head = server.join().unwrap().to_lowercase();
        assert!(
            head.contains(&format!("user-agent: {}", USER_AGENT.to_lowercase())),
            "{head}"
        );
    }

    #[test]
    fn a_registry_that_doesnt_answer_is_an_error_after_the_timeout() {
        // Accepts the connection, never answers
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/versions", listener.local_addr().unwrap());

        let started = std::time::Instant::now();
        let answer = UpdateClient::get::<serde_json::Value>(
            &url,
            Duration::from_millis(200),
            HeaderMap::default(),
        );

        assert!(answer.is_err());
        assert!(started.elapsed() < Duration::from_secs(5));
        drop(listener);
    }
}
