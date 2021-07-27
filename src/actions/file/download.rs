use super::FileAction;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use anyhow::Result;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::{path::PathBuf, u32};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileDownload {
    pub from: String,
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,
}

fn from_octal<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let chmod: u32 = Deserialize::deserialize(deserializer)?;
    u32::from_str_radix(&chmod.to_string(), 8).map_err(D::Error::custom)
}

fn default_chmod() -> u32 {
    0o644
}

fn default_template() -> bool {
    false
}

impl FileDownload {}

impl FileAction for FileDownload {}

impl Action for FileDownload {
    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Chmod;
        use crate::atoms::http::Download;

        let path = PathBuf::from(&self.to);
        let parent = path.clone();

        vec![
            Step {
                atom: Box::new(DirCreate {
                    path: parent.parent().unwrap().into(),
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
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.download
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

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
}
