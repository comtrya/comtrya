//! Small helpers that several actions use

use crate::contexts::Facts;

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

/// The command that elevates privileges: `privilege` from `Comtrya.yaml`, `sudo` by default
pub fn get_privilege_provider(facts: &Facts) -> String {
    facts.privilege.to_string()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::contexts::privilege::Privilege;

    #[test]
    fn the_configured_privilege_provider_is_used() {
        let facts = Facts {
            privilege: Privilege::Doas,
            ..Default::default()
        };

        assert_eq!("doas", get_privilege_provider(&facts));
    }

    #[test]
    fn sudo_is_the_default_privilege_provider() {
        assert_eq!("sudo", get_privilege_provider(&Facts::default()));
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
