use super::FileAction;
use super::{default_chmod, from_octal};
#[cfg(unix)]
use crate::atoms::file::Chown;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "file.download")]
pub struct FileDownload {
    pub from: String,
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,

    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,

    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,
}

fn default_template() -> bool {
    false
}

impl FileDownload {}

impl FileAction for FileDownload {}

impl Action for FileDownload {
    fn summarize(&self) -> String {
        format!("Downloading file {} to {}", self.from, self.to)
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Chmod;
        use crate::atoms::http::Download;

        let path = PathBuf::from(&self.to);
        let parent = path.clone();

        let steps = vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent
                        .parent()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Failed to get parent directory of path: {}",
                                path.display()
                            )
                        })?
                        .into(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Download {
                    url: self.from.clone(),
                    to: path.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Chmod {
                    path: path.clone(),
                    mode: self.chmod,
                }),
                initializers: vec![],
                finalizers: vec![],
            },
        ];

        #[cfg(unix)]
        {
            let mut steps = steps;
            if let Some(user) = self.owner_user.clone() {
                if let Some(group) = self.owner_group.clone() {
                    steps.push(Step {
                        atom: Box::new(Chown {
                            path: path.clone(),
                            owner: user.clone(),
                            group: group.clone(),
                        }),
                        initializers: vec![],
                        finalizers: vec![],
                    })
                }
            }

            Ok(steps)
        }

        #[cfg(not(unix))]
        {
            Ok(steps)
        }
    }
}

#[cfg(test)]
mod tests {
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

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

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
    #[cfg(unix)]
    fn it_can_be_deserialized_owners() {
        let yaml = r#"
- action: file.download
  from: a
  to: b
  owned_by_user: test
  owned_by_group: test
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

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
        };

        let steps = file_download.plan(&Default::default(), &Default::default());
        assert!(steps.is_ok());
        let steps = steps.unwrap();
        assert_eq!(4, steps.len());
    }
}
