mod providers;
pub use providers::register_providers;
pub use providers::ManifestProvider;

use crate::actions::Actions;
use petgraph::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub depends: Vec<String>,

    #[serde(default)]
    pub actions: Vec<Actions>,

    #[serde(skip)]
    pub root_dir: Option<PathBuf>,

    #[serde(skip)]
    pub dag_index: Option<NodeIndex<u32>>,
}
