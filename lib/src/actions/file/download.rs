//! `file.download`

use super::{change_owner, create_parent, default_chmod, from_octal, FileAction, OctalMode};
use crate::atoms::file::Chmod;
use crate::atoms::http::Download;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{
    actions::{Action, Ignored},
    contexts::Contexts,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Downloads a file, unless it exists already and matches `sha256`, if given
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "file.download")]
pub struct FileDownload {
    /// The URL to download
    #[serde(alias = "source")]
    pub from: String,

    /// Where to save the file
    #[serde(alias = "target")]
    pub to: String,

    /// The permissions of the file, `644` by default
    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    #[schemars(with = "OctalMode")]
    pub chmod: u32,

    /// Ignored with a warning: downloaded files aren't rendered
    #[serde(default = "default_template")]
    pub template: bool,

    /// The owner of the file, on unix. Needs `owned_by_group` too
    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,

    /// The group of the file, on unix. Needs `owned_by_user` too
    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,

    /// Expected SHA-256 of the file in hex. The download fails if it doesn't match, and an
    /// existing file that doesn't match is downloaded again
    pub sha256: Option<String>,
}

/// `template` is off unless set
fn default_template() -> bool {
    false
}

impl FileDownload {
    /// The fields of this action that are set but not used
    fn ignored_fields(&self) -> Vec<Ignored> {
        match self.template {
            true => vec![("template", "downloaded files aren't rendered")],
            false => vec![],
        }
    }

    /// `sha256` in lowercase, or an error if it isn't 64 hex characters
    fn sha256(&self) -> anyhow::Result<Option<String>> {
        let Some(sha256) = &self.sha256 else {
            return Ok(None);
        };

        let sha256 = sha256.trim().to_lowercase();
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(anyhow::anyhow!(
                "sha256 must be 64 hex characters, got '{sha256}'"
            ));
        }

        Ok(Some(sha256))
    }
}

impl FileAction for FileDownload {}

impl Action for FileDownload {
    fn summarize(&self) -> String {
        format!("Downloading file {} to {}", self.from, self.to)
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
        crate::actions::warn_ignored(&self.ignored_fields());
        let sha256 = self.sha256()?;
        let path = PathBuf::from(&self.to);

        let mut steps = vec![
            create_parent(&path)?,
            Step::new(Download {
                url: self.from.clone(),
                to: path.clone(),
                sha256,
            }),
            Step::new(Chmod {
                path: path.clone(),
                mode: self.chmod,
            }),
        ];
        steps.extend(change_owner(&path, &self.owner_user, &self.owner_group));

        Ok(steps)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn template_is_named_as_ignored() {
        let download = |template| super::FileDownload {
            template,
            ..Default::default()
        };

        let fields: Vec<&str> = download(true)
            .ignored_fields()
            .iter()
            .map(|(f, _)| *f)
            .collect();
        assert_eq!(vec!["template"], fields);
        assert!(download(false).ignored_fields().is_empty());
    }

    #[cfg(unix)]
    use crate::actions::file::download::FileDownload;
    #[cfg(unix)]
    use crate::actions::Action;
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.download
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileDownload(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
            }
            _ => {
                panic!("FileDownload didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn source_and_target_are_aliases_for_from_and_to() {
        let yaml = "- action: file.download\n  source: a\n  target: b\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileDownload(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
            }
            _ => panic!("FileDownload didn't deserialize to the correct type"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn it_can_be_deserialized_owners() {
        let yaml = r#"
- action: file.download
  from: a
  to: b
  owned_by_user: test
  owned_by_group: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileDownload(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!("test", action.action.owner_user.unwrap());
                assert_eq!("test", action.action.owner_group.unwrap());
            }
            _ => {
                panic!("FileDownload didn't deserialize to the correct type");
            }
        };
    }

    fn with_sha256(sha256: &str) -> crate::actions::file::download::FileDownload {
        crate::actions::file::download::FileDownload {
            from: String::from("https://example.com/file"),
            to: String::from("file"),
            sha256: Some(sha256.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn sha256_can_be_deserialized() {
        let yaml = "- action: file.download\n  from: a\n  to: b\n  sha256: ABC\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileDownload(action)) => {
                assert_eq!(Some("ABC"), action.action.sha256.as_deref())
            }
            _ => panic!("FileDownload didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn sha256_is_normalized_to_lowercase() {
        let upper = "A".repeat(64);
        assert_eq!(Some("a".repeat(64)), with_sha256(&upper).sha256().unwrap());
        assert_eq!(
            Some("a".repeat(64)),
            with_sha256(&format!(" {upper}\n")).sha256().unwrap()
        );
    }

    #[test]
    fn sha256_must_be_64_hex_characters() {
        assert!(with_sha256("abc").sha256().is_err());
        assert!(with_sha256(&"g".repeat(64)).sha256().is_err());
        assert!(with_sha256(&"a".repeat(65)).sha256().is_err());
    }

    #[test]
    #[cfg(unix)]
    fn contains_chown_step() {
        let file_download = FileDownload {
            from: "test".to_string(),
            to: "abc".to_string(),
            chmod: 1,
            template: false,
            owner_user: Some("test".to_string()),
            owner_group: Some("test".to_string()),
            sha256: None,
        };

        let steps = file_download.plan(&Default::default(), &Default::default());
        assert!(steps.is_ok());
        let steps = steps.unwrap();
        assert_eq!(4, steps.len());
    }
}
