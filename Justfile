clippy:
    cargo clippy --all-targets --all-features -- -D warnings

fmt:
    cargo fmt --all -- --check

deny:
    cargo install --locked cargo-deny
    cargo deny init
    cargo deny check
