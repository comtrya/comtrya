//! Decrypts age-encrypted contents into a file

use crate::atoms::{BoxError, Check, Error, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use age::armor::ArmoredReader;
use age::secrecy::SecretString;
use std::io::Read;
use std::path::PathBuf;

/// Writes decrypted contents to a file, every time. Skipped if they can't be decrypted
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decrypt {
    /// The contents, encrypted with an age passphrase and ASCII armored
    pub encrypted_content: Vec<u8>,
    /// The age passphrase
    pub passphrase: String,
    /// The file to write
    pub path: PathBuf,
}

impl std::fmt::Display for Decrypt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The content needs to be decrypted to {}",
            self.path.as_path().display()
        )
    }
}

impl Change for Decrypt {
    fn check(&self) -> Result<Check> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Check::Drift);
        }

        // Fails early for a wrong passphrase
        self.decrypted()?;

        Ok(Check::Drift)
    }

    fn execute(&self) -> Result<()> {
        std::fs::write(&self.path, self.decrypted()?).map_err(Error::io("write", &self.path))?;

        Ok(())
    }
}

impl Decrypt {
    /// The contents decrypted
    fn decrypted(&self) -> Result<Vec<u8>> {
        decrypt(&self.passphrase, &self.encrypted_content).map_err(|source| Error::Decrypt {
            path: self.path.clone(),
            source,
        })
    }
}

/// Decrypts armored age contents with a passphrase
fn decrypt(passphrase: &str, encrypted_content: &[u8]) -> std::result::Result<Vec<u8>, BoxError> {
    let decryptor = age::Decryptor::new(ArmoredReader::new(encrypted_content))?;
    if !decryptor.is_scrypt() {
        return Err("the contents aren't encrypted with a passphrase".into());
    }

    let mut decrypted = vec![];
    let secret = SecretString::from(passphrase.to_owned());
    let identity = age::scrypt::Identity::new(secret);
    let mut reader = decryptor.decrypt(std::iter::once(&identity as &dyn age::Identity))?;
    reader.read_to_end(&mut decrypted)?;

    Ok(decrypted)
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;
    use pretty_assertions::assert_eq;
    use std::io::Write;

    #[test]
    fn it_can_plan() -> anyhow::Result<()> {
        // encrypt and write to file
        let passphrase = "Teal'c".to_string();
        let content = b"Shol'va";
        let encrypted_content = encrypt(passphrase.to_owned(), content.to_vec())?;

        // prepare atom
        let mut file = NamedTempFile::new()?;
        file.reopen()?;
        file.write_all(&encrypted_content)?;

        let decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase,
        };

        // plan
        assert_eq!(Check::Drift, decrypt.check().unwrap());

        // prepare another atom
        let another_decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase: "fkbr".to_string(),
        };

        // plan
        assert!(another_decrypt.check().is_err());

        Ok(())
    }

    #[test]
    fn it_can_execute() -> anyhow::Result<()> {
        // encrypt and write to file
        let passphrase = "Teal'c".to_string();
        let content = b"Shol'va";
        let encrypted_content = encrypt(passphrase.to_owned(), content.to_vec())?;

        // prepare atom
        let mut file = NamedTempFile::new()?;
        file.reopen()?;
        file.write_all(&encrypted_content)?;

        let decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase,
        };

        // plan, execute
        assert_eq!(Check::Drift, decrypt.check().unwrap());
        assert_eq!(true, decrypt.execute().is_ok());

        Ok(())
    }

    fn encrypt(passphrase: String, content: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let secret = SecretString::from(passphrase);
        let encryptor = age::Encryptor::with_user_passphrase(secret);

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&content)?;
        writer.finish()?;

        Ok(encrypted)
    }
}
