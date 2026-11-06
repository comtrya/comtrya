//! Small helpers that several actions use

use sha2::{Digest, Sha256};
use which::which;

/// Lowercase hex SHA-256 of `data`
pub fn sha256(data: impl AsRef<[u8]>) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The full path of a program in `PATH`
pub fn get_binary_path(binary: &str) -> which::Result<String> {
    let binary = which(binary)?.to_string_lossy().to_string();

    Ok(binary)
}

/// The error and its causes, separated by `: `, like "can't read /etc/x: entity not found".
/// Errors of comtrya don't repeat their cause in their message, so this shows why
pub fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

/// Whether comtrya runs as root, by its uid: also under another name, like FreeBSD's `toor`,
/// and without a passwd entry
#[cfg(unix)]
pub fn running_as_root() -> bool {
    uzers::get_current_uid() == 0
}

/// Windows has no root
#[cfg(not(unix))]
pub fn running_as_root() -> bool {
    false
}

#[cfg(test)]
mod test {
    use super::*;

    /// apt key file names are derived from it, so it must never change.
    #[test]
    fn sha256_is_lowercase_hex() {
        assert_eq!(
            // From the NIST test vectors (FIPS 180-2) expected value is the SHA-256 of "abc"
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            sha256("abc")
        );
    }
}
