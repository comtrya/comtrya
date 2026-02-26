pub mod lua;
use std::{borrow::Cow, ops::Deref, path::PathBuf};

use crate::contexts::Contexts;

use camino::Utf8PathBuf;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use which::which;

pub fn get_binary_path(binary: &str) -> Result<String, anyhow::Error> {
    let binary = which(binary)?.to_string_lossy().to_string();

    Ok(binary)
}

pub fn get_privilege_provider(contexts: &Contexts) -> Option<String> {
    let privilege_provider = contexts.get("privilege").and_then(|s| s.first_key_value());

    if let Some(privilege_provider) = privilege_provider {
        return Some(privilege_provider.1.to_string());
    }

    None
}

#[derive(Eq, PartialEq, Debug, Clone, Default, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CustomPathBuf(pub Utf8PathBuf);

impl JsonSchema for CustomPathBuf {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("CustomPathBuf")
    }

    fn json_schema(gen: &mut SchemaGenerator) -> Schema {
        gen.subschema_for::<PathBuf>()
    }
}

impl Deref for CustomPathBuf {
    type Target = Utf8PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
