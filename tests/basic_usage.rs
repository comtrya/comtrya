use std::{fs::{File, create_dir}, path::{Path, PathBuf}};

use utils::*;
use tempdir::TempDir;
use std::io::Result;

#[path = "./utils.rs"]
mod utils;

enum Entry {
    Dir{name: String, entries: Vec<Entry>},
    File{name: String, content: String},
}

impl Entry {
    fn create_in(self, parent: &Path) -> Result<()> {
        match self {
            Entry::Dir { name, entries } => {
                let new_dir = parent.join(name);
                create_dir(new_dir.clone())?;
                for entry in entries {
                    entry.create_in(&new_dir)?;
                };
            },
            Entry::File{name, content} => {
                use std::io::Write;
                let new_path = parent.join(name);
                let mut f = File::create(new_path.clone())?;
                f.write_all(&content.into_bytes()[..])?;
            }
        }
        Ok(())
    }
}

fn f<I>(name: &'static str, s: I) -> Entry
where
    I: Into<String>
{
    Entry::File{name: name.into(), content: s.into()}
}

fn dir(name: &'static str, entries: Vec<Entry>) -> Entry {
    Entry::Dir{name: name.into(), entries}
}

#[test]
fn prints_help() {
    let t = TempDir::new("comtrya").expect("could not create tempdir");
    let path = t.into_path();
    dir("directory", vec![
        dir("copy", vec![
            dir("files", vec![
                dir("mydir", vec![
                    f("file-a","some content a"),
                    f("file-b", "some other thing"),
                ])
            ]),
            f("main.yaml", r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#)
        ])
    ]).create_in(&path.clone()).expect("should have create stuff");



    let assert = cd(path).run("comtrya directory/copy/main.yaml --dry-run");

    assert.success().stdout(c(r#"INFO manifest_run{manifest="copy"}: Completed"#));

}
