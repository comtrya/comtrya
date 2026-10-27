use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub enum Privilege {
    #[serde(alias = "sudo")]
    #[default]
    Sudo,

    #[serde(alias = "doas")]
    Doas,

    #[serde(alias = "run0")]
    Run0,
}

impl Display for Privilege {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Privilege::Sudo => "sudo".to_string(),
            Privilege::Doas => "doas".to_string(),
            Privilege::Run0 => "run0".to_string(),
        };
        write!(f, "{str}")
    }
}
