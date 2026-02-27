use super::FileAction;
use crate::manifests::Manifest;
use crate::steps::initializers::FileExists;
use crate::steps::initializers::FlowControl::Ensure;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec;
use tracing::error;

// TODO: Next Major Version - Deprecate from and to
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

        if let Some(ref source) = self.source {
            source.to_string()
        } else {
            // .unwrap() is safe here because we already checked for None
            self.from.clone().unwrap()
        }
    }

    fn target(&self) -> String {
        if self.target.is_none() && self.to.is_none() {
            error!("Field 'target' is required for file.link");
        }
        if let Some(ref target) = self.target {
            target.to_string()
        } else {
            // .unwrap() is safe here because we already checked for None
            self.to.clone().unwrap()
        }
    }

    pub fn plan_no_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        match to.parent() {
            Some(parent) => {
                vec![
                    Step {
                        atom: Box::new(DirCreate {
                            path: parent.to_path_buf(),
                        }),
                        initializers: vec![],
                        finalizers: vec![],
                    },
                    Step {
                        atom: Box::new(Link {
                            source: from.to_owned(),
                            target: to,
                        }),
                        initializers: vec![Ensure(Box::new(FileExists(from)))],
                        finalizers: vec![],
                    },
                ]
            }
            None => vec![],
        }
    }

    pub fn plan_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        let mut steps = vec![Step {
            atom: Box::new(DirCreate { path: to.clone() }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if let Ok(paths) = std::fs::read_dir(from) {
            paths.for_each(|path| {
                if let Ok(path) = path {
                    let p = path.path();

                    if let Some(file_name) = p.file_name() {
                        steps.push(Step {
                            atom: Box::new(Link {
                                source: p.clone(),
                                target: to.join(file_name),
                            }),
                            initializers: vec![Ensure(Box::new(FileExists(p.clone())))],
                            finalizers: vec![],
                        })
                    }
                }
            })
        }

        steps
    }
}

impl FileAction for FileLink {}

impl Action for FileLink {
    fn summarize(&self) -> String {
        format!(
            "Linking file {} to {}",
            self.from.clone().unwrap_or(String::from("unknown")),
            self.to.clone().unwrap_or(String::from("unknown"))
        )
    }

    fn plan(&self, manifest: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let from: PathBuf = self.resolve(manifest, self.source().as_str())?;

        let to = PathBuf::from(self.target());

        // Can't walk a file
        if from.is_file() {
            return Ok(FileLink::plan_no_walk(from, to));
        }

        match self.walk_dir {
            false => Ok(FileLink::plan_no_walk(from, to)),
            true => Ok(FileLink::plan_walk(from, to)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        actions::{Action, Actions},
        config::Config,
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

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

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

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

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
            r#where: None,
            root_dir: Some(source_dir.path().to_path_buf()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
            ..Default::default()
        };

        let config = Config::default();

        let contexts = build_contexts(&config);

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

        let steps = file_link_action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), 2);
    }

    #[test]
    fn it_can_walk_link_directories() {
        let source_dir = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(_) => {
                panic!("could not create tempdir");
            }
        }
        .keep();

        // We'll expect 2 extra Atoms
        use rand::RngExt;
        use std::io::Write;

        let mut rng = rand::rng();
        let number_of_files: usize = rng.random_range(3..9);

        for i in 0..number_of_files {
            let path = source_dir.clone().join(format!("{i}.txt"));
            let mut file = std::fs::File::create(path).unwrap();
            writeln!(file, "Random {i}").unwrap();
        }

        let manifest: Manifest = Manifest {
            r#where: None,
            root_dir: Some(source_dir.clone()),
            actions: vec![],
            depends: vec![],
            name: None,
            dag_index: None,
            ..Default::default()
        };

        let config = Config::default();

        let contexts = build_contexts(&config);

        let target: String = source_dir.parent().unwrap().to_str().unwrap().to_string();

        let file_link_action: FileLink = FileLink {
            source: Some(source_dir.to_str().unwrap().to_string()),
            target: Some(target),
            walk_dir: true,
            ..Default::default()
        };

        let steps = file_link_action.plan(&manifest, &contexts).unwrap();
        assert_eq!(steps.len(), number_of_files + 1);
    }
}
