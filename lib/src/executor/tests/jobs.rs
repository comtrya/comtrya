//! Running manifests at the same time with `--jobs`, and steps taking turns on the same file

use super::*;
use pretty_assertions::assert_eq;

/// Serves `hello` on localhost. Each answer waits until `together` requests are open at
/// once, up to a second, else it fails with 500. Returns the URL, and how many requests
/// were open at once at most
fn serve(together: usize) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/file", listener.local_addr().unwrap());
    let open = Arc::new(AtomicUsize::new(0));
    let most = Arc::new(AtomicUsize::new(0));
    let most_seen = Arc::clone(&most);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let (open, most) = (Arc::clone(&open), Arc::clone(&most));
            std::thread::spawn(move || {
                let mut stream = stream.unwrap();
                let _ = stream.read(&mut [0; 1024]);
                let now = open.fetch_add(1, Ordering::SeqCst) + 1;
                most.fetch_max(now, Ordering::SeqCst);
                let deadline = Instant::now() + Duration::from_secs(1);
                while open.load(Ordering::SeqCst) < together && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(10));
                }
                let status = match open.load(Ordering::SeqCst) >= together {
                    true => "200 OK",
                    false => "500 Alone",
                };
                // Keep this request open a moment, so overlapping ones are seen
                std::thread::sleep(Duration::from_millis(200));
                let answer = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello"
                );
                let _ = stream.write_all(answer.as_bytes());
                open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    (url, most_seen)
}

/// A manifest `name` that downloads `url` to `to`
fn download(name: &str, url: &str, to: &Path) -> (String, Manifest) {
    let yaml = format!(
        "actions:\n  - action: file.download\n    from: '{url}'\n    to: '{}'\n",
        to.display()
    );
    let manifest = Manifest {
        name: Some(name.to_string()),
        ..serde_yaml_ng::from_str(&yaml).unwrap()
    };
    (name.to_string(), manifest)
}

/// Runs `manifests` with `jobs`
fn run_jobs(manifests: Vec<(String, Manifest)>, dir: &Path, jobs: usize) -> Report {
    let loaded = Loaded {
        manifests: manifests.into_iter().collect(),
        errors: vec![],
    };
    let options = Options {
        jobs,
        ..Default::default()
    };
    run(loaded, dir, PlanCx::empty(), &options)
}

#[test]
fn with_jobs_independent_manifests_run_at_the_same_time() {
    let dir = tempfile::tempdir().unwrap();
    let (url, _) = serve(2);
    let manifests = vec![
        download("a", &url, &dir.path().join("a")),
        download("b", &url, &dir.path().join("b")),
    ];

    // Each download only succeeds while the other one is open too
    let report = run_jobs(manifests, dir.path(), 2);

    assert_eq!(Report::default(), report);
    assert!(dir.path().join("a").is_file() && dir.path().join("b").is_file());
}

#[test]
fn one_job_runs_one_manifest_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let (url, most) = serve(1);
    let manifests = vec![
        download("a", &url, &dir.path().join("a")),
        download("b", &url, &dir.path().join("b")),
    ];

    run_jobs(manifests, dir.path(), 1);

    assert_eq!(1, most.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn steps_that_write_the_same_file_take_turns() {
    let dir = tempfile::tempdir().unwrap();
    let (url, most) = serve(1);
    let same = dir.path().join("same");
    let manifests = vec![download("a", &url, &same), download("b", &url, &same)];

    let report = run_jobs(manifests, dir.path(), 2);

    assert_eq!(Report::default(), report);
    assert_eq!(1, most.load(std::sync::atomic::Ordering::SeqCst));
}
