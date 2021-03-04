use serde::{Deserialize, Serialize};

// User context provider: understands the user running the command
pub mod user;

pub trait ContextProvider {
    fn get_prefix(&self) -> String;
    fn get_contexts(&self) -> Vec<Context>;
}

pub struct ContextStore {
    pub prefix: String,
    pub contexts: Vec<Context>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Context {
    KeyValueContext(String, String),
    ListContext(String, Vec<String>),
}
