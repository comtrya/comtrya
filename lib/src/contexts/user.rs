use crate::values::Value;
use dirs::{config_dir, data_dir, data_local_dir, document_dir, home_dir};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The user running comtrya, the `user` context
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserFacts {
    pub id: u32,
    pub name: String,
    pub username: String,
    pub home_dir: Option<PathBuf>,
    pub config_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub data_local_dir: Option<PathBuf>,
    pub document_dir: Option<PathBuf>,
}

impl UserFacts {
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

#[cfg(unix)]
fn uid() -> u32 {
    uzers::get_current_uid()
}

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
