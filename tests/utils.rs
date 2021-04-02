#![allow(dead_code)]
use assert_cmd::{assert::Assert, Command};

// re-export for all modules to share
pub use predicates::{prelude::PredicateBooleanExt, str::contains as c};
use std::path::PathBuf;

pub(crate) struct Dir {
    cwd: PathBuf,
    env: String,
}

impl Dir {
    pub fn run(self, cli: &'static str) -> Assert {
        let mut comtrya = Command::cargo_bin("comtrya").unwrap();

        comtrya.current_dir(self.cwd);

        let args = cli.split(' ').skip(1).collect::<Vec<_>>();
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
