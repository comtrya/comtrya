clippy:
    cargo clippy --all-targets --all-features -- -D warnings

fmt:
    cargo fmt --all -- --check

deny:
    cargo install --locked cargo-deny
    cargo deny check

test:
    cargo test -- --nocapture --color=always
