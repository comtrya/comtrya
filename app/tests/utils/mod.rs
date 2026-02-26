#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};
use std::io::Result;
use std::{
    fs::{create_dir, File},
    path::Path,
};

// re-export for all modules to share
use std::path::PathBuf;

pub(crate) struct Dir {
    cwd: PathBuf,
    env: String,
}

impl Dir {
    pub fn run(self, cli: &'static str) -> Assert {
        let mut comtrya = Command::new(assert_cmd::cargo::cargo_bin!("comtrya"));

        comtrya.current_dir(self.cwd);

        let args = cli.split(' ').collect::<Vec<_>>();
        comtrya.args(args);

        comtrya.assert()
    }

    pub fn env<S: Into<String>>(mut self, env: S) -> Dir {
        self.env = env.into();

        self
    }
}

pub(crate) fn run(cli: &'static str) -> Assert {
    cd(PathBuf::from("./")).run(cli)
}

pub(crate) fn cd(path: PathBuf) -> Dir {
    if !path.exists() {
        panic!("could not 'cd' into non-existing file: {:?}", path);
    }

    Dir {
        cwd: path,
        env: "".into(),
    }
}

pub(crate) enum Entry {
    Dir { name: String, entries: Vec<Entry> },
    File { name: String, content: String },
}

impl Entry {
    pub(crate) fn create_in(self, parent: &Path) -> Result<()> {
        match self {
            Entry::Dir { name, entries } => {
                let new_dir = parent.join(name);
                create_dir(new_dir.clone())?;
                for entry in entries {
                    entry.create_in(&new_dir)?;
                }
            }
            Entry::File { name, content } => {
                use std::io::Write;
                let new_path = parent.join(name);
                let mut f = File::create(new_path)?;
                f.write_all(&content.into_bytes()[..])?;
            }
        }
        Ok(())
    }
}

pub(crate) fn f<I>(name: &'static str, s: I) -> Entry
where
    I: Into<String>,
{
    Entry::File {
        name: name.into(),
        content: s.into(),
    }
}

pub(crate) fn dir(name: &'static str, entries: Vec<Entry>) -> Entry {
    Entry::Dir {
        name: name.into(),
        entries,
    }
}
