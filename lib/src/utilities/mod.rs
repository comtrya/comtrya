//! Small helpers that several actions use

use crate::contexts::{privilege::Privilege, Contexts};

use sha2::{Digest, Sha256};
use which::which;

/// Lowercase hex SHA-256 of `data`
pub fn sha256(data: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(data))
}

/// The full path of a program in `PATH`
pub fn get_binary_path(binary: &str) -> Result<String, anyhow::Error> {
    let binary = which(binary)?.to_string_lossy().to_string();

    Ok(binary)
}

/// The command that elevates privileges: the `privilege` context set from `Comtrya.yaml`,
/// or `sudo` without one
pub fn get_privilege_provider(contexts: &Contexts) -> String {
    contexts
        .get("privilege")
        .and_then(|privilege| privilege.first_key_value())
        .map(|(_, provider)| provider.to_string())
        .unwrap_or_else(|| Privilege::Sudo.to_string())
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
    use std::collections::BTreeMap;

    #[test]
    fn the_configured_privilege_provider_is_used() {
        let mut contexts = Contexts::new();
        contexts.insert(
            String::from("privilege"),
            BTreeMap::from([(String::from("privilege"), String::from("doas").into())]),
        );

        assert_eq!("doas", get_privilege_provider(&contexts));
    }

    #[test]
    fn sudo_is_the_default_privilege_provider() {
        assert_eq!("sudo", get_privilege_provider(&Contexts::new()));
    }

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
