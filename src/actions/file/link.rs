use super::FileAction;
use crate::actions::Action;
use crate::{actions::ActionAtom, manifests::Manifest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileLink {
    pub from: String,
    pub to: String,
}

impl FileLink {}

impl FileAction for FileLink {}

impl Action for FileLink {
    fn plan(&self, _: &Manifest, _: &Context) -> Vec<ActionAtom> {
        use crate::atoms::command::Exec;
        use crate::atoms::file::Link;

        let from = PathBuf::from(&self.from);
        let to = PathBuf::from(&self.to);
        let parent = from.clone();

        vec![
            ActionAtom {
                atom: Box::new(Exec {
                    command: String::from("mkdir"),
                    arguments: vec![
                        String::from("-p"),
                        String::from(parent.parent().unwrap().to_str().unwrap()),
                    ],
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            ActionAtom {
                atom: Box::new(Link { from, to }),
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
- action: file.link
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(link)) => {
                assert_eq!("a", link.from);
                assert_eq!("b", link.to);
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };
    }
}
