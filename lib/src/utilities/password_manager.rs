use std::{process::Stdio, sync::Arc};

use anyhow::Result;
use rpassword::prompt_password;
use tokio::{
    io::AsyncWriteExt,
    process::Command,
    time::{interval, Duration},
};
use tracing::error;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Debug, Zeroize, ZeroizeOnDrop, Clone)]
pub struct PasswordManager {
    privilege_provider: String,
    #[cfg(unix)]
    pub secret: Option<Zeroizing<String>>,
    #[cfg(windows)]
    token: String,
}

impl PasswordManager {
    pub fn new(package_providor: Option<&str>) -> Result<Self> {
        let this = Self {
            privilege_provider: package_providor.unwrap_or_default().to_string(),
            #[cfg(unix)]
            secret: None,
            #[cfg(windows)]
            token: String::new(),
        };

        Ok(this)
    }

    #[cfg(target_os = "linux")]
    pub async fn prompt(&mut self, prompt: &str) -> Result<()> {
        self.secret = Some(Zeroizing::new(prompt_password(prompt)?));
        self.keep_elevated().await;
        Ok(())
    }

    async fn keep_elevated(&self) {
        let shared_self = Arc::new(self.clone());
        let mut wait = interval(Duration::from_secs(60 * 10));
        tokio::spawn(async move {
            let this = Arc::clone(&shared_self);
            let mut command = Command::new(this.privilege_provider.clone())
                .arg("-S") // Read password from stdin
                .arg("-v") // Validate and update timestamp
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to refresh sudo");

            if let (Some(mut stdin), Some(secret)) = (command.stdin.take(), this.secret.clone()) {
                stdin
                    .write_all(format!("{}\n", secret.as_str()).as_bytes())
                    .await
                    .unwrap();
            } else {
                error!("Unable to elevate privilege")
            }

            wait.tick().await;
        });
    }
}
