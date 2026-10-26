use crate::contexts::{Context, ContextProvider};
use crate::values::Value;
use anyhow::Result;
use dirs::{config_dir, data_dir, data_local_dir, document_dir, home_dir};
use std::path::PathBuf;

pub struct UserContextProvider {}

impl ContextProvider for UserContextProvider {
    fn get_prefix(&self) -> String {
        String::from("user")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let name = whoami::realname().unwrap_or_else(|_| String::from("unknown"));
        let username = whoami::username().unwrap_or_else(|_| String::from("unknown"));

        Ok(vec![
            Context::KeyValueContext(String::from("id"), self.get_uid().to_string().into()),
            Context::KeyValueContext(String::from("name"), name.into()),
            Context::KeyValueContext(String::from("username"), username.into()),
            Context::KeyValueContext(String::from("home_dir"), dir(home_dir())),
            Context::KeyValueContext(String::from("config_dir"), dir(config_dir())),
            Context::KeyValueContext(String::from("data_dir"), dir(data_dir())),
            Context::KeyValueContext(String::from("data_local_dir"), dir(data_local_dir())),
            Context::KeyValueContext(String::from("document_dir"), dir(document_dir())),
        ])
    }
}

fn dir(path: Option<PathBuf>) -> Value {
    path.map(|path| template_path(&path.display().to_string()).into())
        .unwrap_or_else(|| "unknown".into())
}

/// Windows accepts `/` as a separator. Backslashes would break manifests that put the path in a
/// double-quoted YAML string, because templates are rendered first: `"C:\Users"` has an invalid
/// `\U` escape
fn template_path(path: &str) -> String {
    if cfg!(windows) {
        path.replace('\\', "/")
    } else {
        path.to_string()
    }
}

impl UserContextProvider {
    #[cfg(unix)]
    fn get_uid(&self) -> u32 {
        uzers::get_current_uid()
    }

    #[cfg(not(unix))]
    fn get_uid(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(windows)]
    fn windows_paths_use_forward_slashes() {
        assert_eq!(
            "C:/Users/me/AppData",
            template_path("C:\\Users\\me\\AppData")
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn unix_paths_are_unchanged() {
        assert_eq!("/home/me/odd\\name", template_path("/home/me/odd\\name"));
    }
}
