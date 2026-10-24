$ErrorActionPreference = "Stop"

cargo fmt
cargo clippy --all-features --all-targets -- -D warnings
cargo test
