use serde::{Deserialize, Serialize};

fn get_true() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub from: Option<String>,
    pub to: Option<String>,

    #[serde(default = "get_true")]
    pub template: bool,

    #[serde(default)]
    pub symlink: Option<bool>,

    #[serde(default)]
    pub force: bool,

    #[serde(default)]
    pub omit: Vec<String>,
}
