use std::{error::Error, fs::create_dir_all, path::PathBuf};

use crate::files::File;
use crate::packages::PackageConfig;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,

    #[serde(default)]
    pub depends: Vec<String>,

    #[serde(default)]
    pub packages: Vec<PackageConfig>,

    #[serde(default)]
    pub files: Vec<File>,

    #[serde(default)]
    pub name: Option<String>,
}

impl Manifest {
    pub fn render(&self, file: File, tera: &Tera, context: &Context) -> String {
        tera.clone()
            .render(
                format!(
                    "{}/{}",
                    self.root_dir.clone().unwrap().to_str().unwrap(),
                    file.from.clone().unwrap()
                )
                .as_str(),
                &context,
            )
            .unwrap()
    }

    pub fn create(&self, file: File, tera: &Tera, context: &Context) -> Result<(), Box<dyn Error>> {
        println!(
            "Creating file {:?}",
            self.root_dir
                .clone()
                .unwrap()
                .join(file.clone().to.unwrap())
        );
        let mut parent = self
            .root_dir
            .clone()
            .unwrap()
            .join(file.clone().to.unwrap());

        parent.pop();

        println!("Creating directory {:?}", parent.clone().to_str());
        create_dir_all(parent)?;

        let mut f = std::fs::File::create(
            self.root_dir
                .clone()
                .unwrap()
                .join(file.clone().to.unwrap()),
        )?;

        use std::io::Write;
        if true == file.template {
            f.write_all(self.render(file.clone(), tera, context).as_bytes())?;
        } else {
            f.write_all(
                std::fs::read_to_string(PathBuf::from(
                    tera.get_template(
                        self.root_dir
                            .clone()
                            .unwrap()
                            .join(file.clone().from.unwrap().as_str())
                            .to_str()
                            .unwrap(),
                    )
                    .unwrap()
                    .path
                    .clone()
                    .unwrap(),
                ))
                .unwrap()
                .as_bytes(),
            )
            .unwrap();
        }

        f.sync_all()?;

        Ok(())
    }

    pub fn link(&self, file: File, tera: &Tera) -> Result<(), Box<dyn Error>> {
        println!(
            "Symlinking file {:?} to {:?}",
            self.root_dir
                .clone()
                .unwrap()
                .join(file.from.clone().unwrap()),
            file.clone().to.unwrap()
        );

        std::os::unix::fs::symlink(
            PathBuf::from(
                tera.get_template(
                    self.root_dir
                        .clone()
                        .unwrap()
                        .join(file.clone().from.unwrap().as_str())
                        .to_str()
                        .unwrap(),
                )
                .unwrap()
                .path
                .clone()
                .unwrap(),
            ),
            PathBuf::from(file.to.unwrap()),
        )?;

        Ok(())
    }
}
