use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context, Result};
use rpassword::prompt_password;
use tracing::warn;
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn prompt(&mut self, prompt: &str) -> Result<()> {
        let attempts = 3;

        for attempt in 1..=attempts {
            let secret = Zeroizing::new(prompt_password(prompt)?);

            if !self.try_password(&secret)? {
                warn!("Incorrect Password. Try again! (attempt: {attempt}/{attempts})");
                continue;
            }

            self.secret = Some(secret);
            return Ok(());
        }

        Err(anyhow!("Too many incorrect attempts. Access denied."))
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn try_password(&self, secret: &Zeroizing<String>) -> Result<bool> {
        let mut pass_cmd = Command::new("sudo")
            .arg("-Sv")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        pass_cmd
            .stdin
            .take()
            .context("Error occured while attempting pasword verificaton")?
            .write_all(format!("{}\n", secret.as_str()).as_bytes())?;

        let output = pass_cmd.wait_with_output()?;
        println!(
            "{}, {}",
            unsafe { String::from_utf8_unchecked(output.stdout) },
            unsafe { String::from_utf8_unchecked(output.stderr) },
        );
        Ok(output.status.success())
    }
}
