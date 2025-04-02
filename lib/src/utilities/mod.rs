pub mod password_manager;

use crate::contexts::Contexts;
use which::which;

pub fn get_binary_path(binary: &str) -> Result<String, anyhow::Error> {
    let binary = which(String::from(binary))?.to_string_lossy().to_string();

    Ok(binary)
}

pub fn get_privilege_provider(contexts: &Contexts) -> Option<String> {
    let privilege_provider = contexts.get("privilege").and_then(|s| s.first_key_value());

    if let Some(privilege_provider) = privilege_provider {
        return Some(privilege_provider.1.to_string());
    }

    None
}
