use anyhow::Result;
use rpassword::prompt_password;
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
    pub fn prompt(&mut self, prompt: &str) -> Result<()> {
        self.secret = Some(Zeroizing::new(prompt_password(prompt)?));
        Ok(())
    }
}
