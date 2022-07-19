#!/usr/bin/env bash
set -euo pipefail

cargo fmt
cargo clippy
cargo nextest run
