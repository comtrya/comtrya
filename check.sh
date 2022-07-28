#!/usr/bin/env bash
set -euo pipefail

cargo fmt
cargo clippy --tests
cargo nextest run
