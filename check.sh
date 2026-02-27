#!/usr/bin/env bash
set -euo pipefail

cargo fmt
cargo clippy --all-features --all-targets
cargo test 
