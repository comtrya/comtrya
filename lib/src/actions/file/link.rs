//! `file.link`: symlinks to files in the manifest's `files` directory

use super::FileAction;
use crate::manifests::Manifest;
use crate::steps::initializers::FileExists;
use crate::steps::initializers::FlowControl::Ensure;
use crate::steps::Step;
use crate::{actions::Action, contexts::Contexts};
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::vec;

// TODO: Next Major Version - Deprecate from and to
/// Links `target` to `source`. With `walk_dir`, links each entry of the `source` directory
/// instead
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLink {
    /// Old name of `source`
    pub from: Option<String>,
    /// The file or directory to link to, below the manifest's `files` directory
    pub source: Option<String>,

    /// Where to create the link
    pub target: Option<String>,
    /// Old name of `target`
    pub to: Option<String>,

    /// Link each entry of the `source` directory into the `target` directory, instead of the
    /// directory itself
    #[serde(default = "walk_dir_default")]
    pub walk_dir: bool,
}

/// Directories are linked as a whole unless `walk_dir` says otherwise
fn walk_dir_default() -> bool {
    false
}

impl FileLink {
    /// `source`, or the old `from`
    fn source(&self) -> anyhow::Result<&str> {
        self.source
            .as_deref()
            .or(self.from.as_deref())
            .ok_or_else(|| anyhow!("file.link needs `source`"))
    }

    /// `target`, or the old `to`
    fn target(&self) -> anyhow::Result<&str> {
        self.target
            .as_deref()
            .or(self.to.as_deref())
            .ok_or_else(|| anyhow!("file.link needs `target`"))
    }

    /// Creates the directory of the link, and the link, if `from` exists
    pub fn plan_no_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        match to.parent() {
            Some(parent) => {
                vec![
                    Step::new(DirCreate {
                        path: parent.to_path_buf(),
                    }),
                    Step::new(Link {
                        source: from.to_owned(),
                        target: to,
                    })
                    .with_initializer(Ensure(Box::new(FileExists(from)))),
                ]
            }
            None => vec![],
        }
    }

    /// Creates `to`, and in it a link to each entry of `from`
    pub fn plan_walk(from: PathBuf, to: PathBuf) -> Vec<Step> {
        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::Link;

        let mut steps = vec![Step::new(DirCreate { path: to.clone() })];

        if let Ok(paths) = std::fs::read_dir(from) {
            paths.for_each(|path| {
                if let Ok(path) = path {
                    let p = path.path();

                    if let Some(file_name) = p.file_name() {
                        steps.push(
                            Step::new(Link {
                                source: p.clone(),
                                target: to.join(file_name),
                            })
                            .with_initializer(Ensure(Box::new(FileExists(p.clone())))),
                        )
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
            self.source().unwrap_or("unknown"),
            self.target().unwrap_or("unknown")
        )
    }

    fn plan(&self, manifest: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let from: PathBuf = self.resolve(manifest, self.source()?)?;

        let to = PathBuf::from(self.target()?);

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
                assert_eq!("a", action.action.source().unwrap());
                assert_eq!("b", action.action.target().unwrap());
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
                assert_eq!("a", action.action.source().unwrap());
                assert_eq!("b", action.action.target().unwrap());
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

        use std::io::Write;

        let number_of_files: usize = 5;

        for i in 0..number_of_files {
            let path = source_dir.clone().join(format!("{i}.txt"));
            let mut file = std::fs::File::create(path).unwrap();
            writeln!(file, "File {i}").unwrap();
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
        // One link per file, plus creating the target directory
        assert_eq!(steps.len(), number_of_files + 1);
    }

    #[test]
    fn summary_names_source_and_target_in_both_spellings() {
        let link = |yaml: &str| -> FileLink { serde_yaml_ng::from_str(yaml).unwrap() };

        assert_eq!(
            "Linking file a to b",
            link("source: a\ntarget: b\n").summarize()
        );
        assert_eq!("Linking file a to b", link("from: a\nto: b\n").summarize());
    }
}
