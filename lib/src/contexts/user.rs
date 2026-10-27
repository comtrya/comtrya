//! The `user` context

use crate::values::Value;
use dirs::{config_dir, data_dir, data_local_dir, document_dir, home_dir};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The user running comtrya, the `user` context
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserFacts {
    /// The uid. 0 on Windows
    pub id: u32,
    /// The real name
    pub name: String,
    /// The login name
    pub username: String,
    /// The home directory
    pub home_dir: Option<PathBuf>,
    /// Where configuration goes, like `~/.config`
    pub config_dir: Option<PathBuf>,
    /// Where data goes, like `~/.local/share`
    pub data_dir: Option<PathBuf>,
    /// Where data that stays on this machine goes
    pub data_local_dir: Option<PathBuf>,
    /// The documents directory
    pub document_dir: Option<PathBuf>,
}

impl UserFacts {
    /// Looks at the user running comtrya
    pub fn gather() -> Self {
        Self {
            id: uid(),
            name: whoami::realname().unwrap_or_else(|_| String::from("unknown")),
            username: whoami::username().unwrap_or_else(|_| String::from("unknown")),
            home_dir: home_dir(),
            config_dir: config_dir(),
            data_dir: data_dir(),
            data_local_dir: data_local_dir(),
            document_dir: document_dir(),
        }
    }

    /// The `user` context. Missing directories are `unknown`
    pub fn to_context(&self) -> BTreeMap<String, Value> {
        BTreeMap::from([
            (String::from("id"), self.id.to_string().into()),
            (String::from("name"), self.name.clone().into()),
            (String::from("username"), self.username.clone().into()),
            (String::from("home_dir"), dir(&self.home_dir)),
            (String::from("config_dir"), dir(&self.config_dir)),
            (String::from("data_dir"), dir(&self.data_dir)),
            (String::from("data_local_dir"), dir(&self.data_local_dir)),
            (String::from("document_dir"), dir(&self.document_dir)),
        ])
    }
}

/// A directory as a value for templates, or `unknown`
fn dir(path: &Option<PathBuf>) -> Value {
    path.as_ref()
        .map(|path| template_path(&path.display().to_string()).into())
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

/// The uid of the user running comtrya
#[cfg(unix)]
fn uid() -> u32 {
    uzers::get_current_uid()
}

/// Windows has no uids
#[cfg(not(unix))]
fn uid() -> u32 {
    0
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
