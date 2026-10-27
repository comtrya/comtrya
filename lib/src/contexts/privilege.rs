//! The program that runs privileged commands

use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The program that runs privileged commands, `privilege` in `Comtrya.yaml`
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub enum Privilege {
    /// `sudo`, the default
    #[serde(alias = "sudo")]
    #[default]
    Sudo,

    /// `doas`
    #[serde(alias = "doas")]
    Doas,

    /// `run0` from systemd
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
