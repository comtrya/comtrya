use crate::atoms::Outcome;

use super::super::Atom;
use super::FileAtom;
use crate::utilities::password_manager::PasswordManager;
use age::armor::ArmoredReader;
use age::secrecy::Secret;
use std::io::Read;
use std::path::PathBuf;
use tracing::error;

pub struct Decrypt {
    pub encrypted_content: Vec<u8>,
    pub passphrase: String,
    pub path: PathBuf,
}

impl FileAtom for Decrypt {
    fn get_path(&self) -> &PathBuf {
        &self.path
    }
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

#[async_trait::async_trait]
impl Atom for Decrypt {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // If the file doesn't exist, assume it's because
        // another atom is going to provide it.
        if !self.path.exists() {
            return Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            });
        }

        // Decrypting file with provided passphrase makes plan work
        match decrypt(&self.passphrase, &self.encrypted_content) {
            Ok(_) => Ok(Outcome {
                side_effects: vec![],
                should_run: true,
            }),
            Err(err) => {
                error!(
                    "Cannot decrypt file {} because {:?}. Skipping.",
                    self.path.display(),
                    err
                );

                Ok(Outcome {
                    side_effects: vec![],
                    should_run: false,
                })
            }
        }
    }

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        let decrypted_content = decrypt(&self.passphrase, &self.encrypted_content)?;

        std::fs::write(&self.path, decrypted_content)?;

        Ok(())
    }
}

fn decrypt(passphrase: &str, encrypted_content: &[u8]) -> anyhow::Result<Vec<u8>> {
    let decryptor = match age::Decryptor::new(ArmoredReader::new(encrypted_content))? {
        age::Decryptor::Passphrase(d) => Ok(d),
        _ => Err(anyhow::anyhow!("Cannot create passphrase decryptor!")),
    }?;

    let mut decrypted = vec![];
    let secret = Secret::new(passphrase.to_owned());
    let mut reader = decryptor.decrypt(&secret, None)?;
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
        assert_eq!(true, decrypt.plan().unwrap().should_run);

        // prepare another atom
        let another_decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase: "fkbr".to_string(),
        };

        // plan
        assert_eq!(false, another_decrypt.plan().unwrap().should_run);

        Ok(())
    }

    #[tokio::test]
    async fn it_can_execute() -> anyhow::Result<()> {
        // encrypt and write to file
        let passphrase = "Teal'c".to_string();
        let content = b"Shol'va";
        let encrypted_content = encrypt(passphrase.to_owned(), content.to_vec())?;

        // prepare atom
        let mut file = NamedTempFile::new()?;
        file.reopen()?;
        file.write_all(&encrypted_content)?;

        let mut decrypt = Decrypt {
            encrypted_content: encrypted_content.to_owned(),
            path: file.path().to_path_buf(),
            passphrase,
        };

        // plan, execute
        assert_eq!(true, decrypt.plan().unwrap().should_run);
        assert_eq!(true, decrypt.execute(None).await.is_ok());

        Ok(())
    }

    fn encrypt(passphrase: String, content: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let secret = Secret::new(passphrase);
        let encryptor = age::Encryptor::with_user_passphrase(secret);

        let mut encrypted = vec![];
        let mut writer = encryptor.wrap_output(&mut encrypted)?;
        writer.write_all(&content)?;
        writer.finish()?;

        Ok(encrypted)
    }
}
