$ErrorActionPreference = "Stop"

cargo fmt
cargo clippy --all-features --all-targets
cargo test
