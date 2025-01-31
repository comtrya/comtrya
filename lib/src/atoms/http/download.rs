use crate::atoms::Outcome;
use crate::utilities::password_manager::PasswordManager;

use super::super::Atom;
use std::io::Write;
use std::{fs::File, path::PathBuf};

pub struct Download {
    pub url: String,
    pub to: PathBuf,
}

impl std::fmt::Display for Download {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpDownload from {} to {}", self.url, self.to.display())
    }
}

#[async_trait::async_trait]
impl Atom for Download {
    fn plan(&self) -> anyhow::Result<Outcome> {
        // Initial implementation will return false if the local file
        // doesn't exist. I'd like to include a SHA to verify the
        // correct version exists; or perhaps a TTL when omitted?

        Ok(Outcome {
            side_effects: vec![],
            should_run: !PathBuf::from(&self.to).exists(),
        })
    }

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        let response = reqwest::blocking::get(&self.url)?;

        let mut file = File::create(&self.to)?;

        let content = response.bytes()?;
        file.write_all(&content)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[tokio::test]
    async fn it_can() {
        let tmpdir = tempdir().unwrap();
        let to_file = tmpdir.path().join("download");

        let mut atom = Download {
            url: String::from("https://www.google.com/images/branding/googlelogo/2x/googlelogo_color_272x92dp.png"),
            to: to_file,
        };

        assert_eq!(true, atom.plan().unwrap().should_run);

        let result = atom.execute(PasswordManager::new(None).ok()).await;
        assert_eq!(true, result.is_ok());
        assert_eq!(false, atom.plan().unwrap().should_run);
    }
}
