use std::{error::Error, path::PathBuf};

use crate::files::File;
use crate::packages::Package;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Module {
    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,

    #[serde(default)]
    pub depends: Vec<String>,

    #[serde(default)]
    pub packages: Vec<Package>,

    #[serde(default)]
    pub files: Vec<File>,

    #[serde(default)]
    pub name: Option<String>,
}

impl Module {
    pub fn render(&self, file: File) -> String {
        let tera = match Tera::new(self.root_dir.clone().unwrap().to_str().unwrap()) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };

        let context = Context::new();
        let contents = file.from.clone().unwrap();
        let template = contents.as_str();

        tera.render(template, &context).unwrap()
    }

    pub fn create(&self, file: File) -> Result<(), Box<dyn Error>> {
        println!(
            "Creating file {:?}",
            self.root_dir
                .clone()
                .unwrap()
                .join(file.clone().to.unwrap())
        );
        let mut f = std::fs::File::create(
            self.root_dir
                .clone()
                .unwrap()
                .join(file.clone().to.unwrap()),
        )?;

        use std::io::Write;
        f.write_all(self.render(file.clone()).as_bytes())?;

        f.sync_all()?;

        Ok(())
    }

    pub fn link(&self, file: File) -> Result<(), Box<dyn Error>> {
        println!(
            "Symlining file {:?} to {:?}",
            self.root_dir
                .clone()
                .unwrap()
                .join(file.from.clone().unwrap()),
            file.clone().to.unwrap()
        );

        std::os::unix::fs::symlink(
            PathBuf::from(self.root_dir.clone().unwrap().join(file.from.unwrap())),
            PathBuf::from(file.to.unwrap()),
        )?;

        Ok(())
    }
}
