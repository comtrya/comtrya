use super::FileAction;
use crate::manifests::Manifest;
use crate::steps::initializers::FileExists;
use crate::steps::initializers::FlowControl::Ensure;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::error;

// TODO: Next Major Version - Deprecate from and to
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FileLink {
    pub from: Option<String>,
    pub source: Option<String>,

    pub target: Option<String>,
    pub to: Option<String>,

    #[serde(default = "walk_dir_default")]
    pub walk_dir: bool,
}

fn walk_dir_default() -> bool {
    false
}

impl FileLink {
    fn source(&self) -> String {
        if self.source.is_none() && self.from.is_none() {
            error!("Field 'source' is required for file.link");
        }
        if self.source.is_none() {
            return self.from.clone().unwrap();
        }

        return self.source.clone().unwrap();
    }

    fn target(&self) -> String {
        if self.target.is_none() && self.to.is_none() {
            error!("Field 'target' is required for file.link");
        }
        if self.target.is_none() {
            return self.to.clone().unwrap();
        }

        return self.target.clone().unwrap();
    }

    pub fn plan_no_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        vec![
            Step {
                atom: Box::new(DirCreate {
                    path: to.parent().unwrap().to_path_buf(),
                }),
                initializers: vec![],
                finalizers: vec![],
            },
            Step {
                atom: Box::new(Link {
                    source: from.to_owned(),
                    target: to,
                }),
                initializers: vec![Ensure(Box::new(FileExists(from.to_owned())))],
                finalizers: vec![],
            },
        ]
    }

    pub fn plan_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        let mut steps = vec![Step {
            atom: Box::new(DirCreate {
                path: to.to_path_buf(),
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        let paths = std::fs::read_dir(from).unwrap();

        steps.extend(paths.map(|path| {
            let p = path.unwrap().path().to_path_buf();

            Step {
                atom: Box::new(Link {
                    source: p.clone(),
                    target: to.join(p.file_name().unwrap()),
                }),
                initializers: vec![Ensure(Box::new(FileExists(p.clone())))],
                finalizers: vec![],
            }
        }));

        steps
    }
}

impl FileAction for FileLink {}

impl Action for FileLink {
    fn plan(&self, manifest: &Manifest, _: &Contexts) -> Vec<Step> {
        let from: PathBuf = self.resolve(manifest, self.source().as_str());
        let to = PathBuf::from(self.target());

        // Can't walk a file
        if from.is_file() {
            return FileLink::plan_no_walk(from, to);
        }

        match self.walk_dir {
            false => FileLink::plan_no_walk(from, to),
            true => FileLink::plan_walk(from, to),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::{Action, Actions},
        contexts::build_contexts,
        manifests::Manifest,
    };

    use super::FileLink;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.link
  source: a
  target: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source());
                assert_eq!("b", action.action.target());
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };

        // Old style format
        let yaml = r#"
- action: file.link
  from: a
  to: b
"#;

        let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileLink(action)) => {
                assert_eq!("a", action.action.source());
                assert_eq!("b", action.action.target());
            }
            _ => {
                panic!("FileLink didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_links_directories() {
        let source_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        };

        let manifest: Manifest = Manifest {
            root_dir: Some(source_dir.path().to_path_buf()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
        };

        let contexts = build_contexts();

        let target: String = source_dir
            .path()
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let file_link_action: FileLink = FileLink {
            source: Some(source_dir.path().to_str().unwrap().to_string()),
            target: Some(target),
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts);
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn it_can_walk_link_directories() {
        let source_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                assert_eq!(false, true);
                return;
            }
        }
        .into_path();

        // We'll expect 2 extra Atoms
        use rand::Rng;
        use std::io::Write;

        let mut rng = rand::thread_rng();
        let number_of_files: usize = rng.gen_range(3..9);

        for i in 0..number_of_files {
            let path = source_dir.clone().join(format!("{}.txt", i));
            let mut file = std::fs::File::create(path).unwrap();
            writeln!(file, "Random {}", i).unwrap();
            println!("Done {}", i);
        }

        let manifest: Manifest = Manifest {
            root_dir: Some(source_dir.clone()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
        };

        let contexts = build_contexts();

        let target: String = source_dir
            .clone()
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let file_link_action: FileLink = FileLink {
            source: Some(source_dir.clone().to_str().unwrap().to_string()),
            target: Some(target),
            walk_dir: true,
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts);
        assert_eq!(steps.len(), number_of_files + 1);
    }
}
