//! The crypto provider for TLS: ring, installed once for the whole process
//!
//! reqwest is built without a provider of its own (`rustls-no-provider`), because its default,
//! aws-lc, takes the longest of all crates to build. Without an installed provider, reqwest
//! panics as soon as a client is built, even for a plain `http://` URL. So every place that
//! talks to the network calls [`install_crypto_provider`] first: downloads, GitHub releases, git
//! clones (gix builds its own reqwest client) and the app's update check.

use std::sync::Once;

/// Installs ring as the process-wide rustls crypto provider, if none is installed yet.
///
/// Safe to call any number of times and from any thread. A provider that something else
/// installed first stays: rustls allows only one, and any provider works for comtrya
pub fn install_crypto_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        // An error only means another provider was installed first, which is fine
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_can_be_built_after_installing() {
        install_crypto_provider();
        install_crypto_provider();

        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
        assert!(reqwest::Client::builder().build().is_ok());
    }
}
