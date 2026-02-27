use crate::config::Config;
use crate::contexts::{Context, ContextProvider};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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

pub struct PrivilegeContextProvider<'a> {
    pub config: &'a Config,
}

impl<'a> ContextProvider for PrivilegeContextProvider<'a> {
    fn get_prefix(&self) -> String {
        "privilege".to_string()
    }

    fn get_contexts(&self) -> anyhow::Result<Vec<Context>> {
        let contexts = vec![Context::KeyValueContext(
            "privilege".to_string(),
            self.config.privilege.to_string().into(),
        )];

        Ok(contexts)
    }
}
