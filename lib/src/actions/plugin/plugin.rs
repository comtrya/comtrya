use std::{
    fmt::{self, Display},
    fs::read_to_string,
    hash::{Hash, Hasher},
    ops::Deref,
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use dirs_next::data_local_dir;
use gix::{
    bstr::ByteSlice, diff::object::tree::EntryKind, interrupt::IS_INTERRUPTED, open,
    prepare_clone_bare, progress::Discard,
};
use schemars::{gen::SchemaGenerator, schema::Schema, JsonSchema};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;
#[allow(unused_imports)]
use serde_with::{formats::PreferOne, serde_as, KeyValueMap, OneOrMany};

use super::{get_plugin, PluginRuntimeSpec};
use crate::{
    actions::Action, atoms::plugin::PluginExec, contexts::Contexts, manifests::Manifest,
    steps::Step, utilities::lua::json_to_lua_value,
};

#[derive(Eq, PartialEq, Debug, Clone, Default, Serialize, Deserialize, PartialOrd, Ord)]
pub(crate) struct CustomPathBuf(Utf8PathBuf);

impl JsonSchema for CustomPathBuf {
    fn schema_name() -> String {
        String::from("CustomPathBuf")
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

#[derive(JsonSchema, Debug, Serialize, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct RepoInfo {
    username: String,
    repo: String,
}

impl<'de> Deserialize<'de> for RepoInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let original = String::deserialize(deserializer)?;

        let Some((username, repo)) = original.split_once('/') else {
            return Err(serde::de::Error::custom(
                "repo must be in format 'username/repo'",
            ));
        };

        Ok(Self {
            username: username.to_string(),
            repo: repo.to_string(),
        })
    }
}

impl RepoInfo {
    pub fn uri(&self) -> String {
        format!("{}/{}", &self.username, self.repo)
    }
}

pub trait Source {
    fn source(&self) -> Result<String>;
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, Default, Eq, PartialOrd, Ord)]
pub struct Repo {
    #[serde(alias = "repository")]
    repo: RepoInfo,

    #[serde(alias = "tag", default)]
    version: String,
}

impl PartialEq for Repo {
    fn eq(&self, other: &Self) -> bool {
        self.path().ok() == other.path().ok()
    }
}

impl Hash for Repo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Ok(path) = self.path() {
            path.hash(state);
        } else {
            self.repo.uri().hash(state);
            self.version.hash(state);
        }
    }
}

impl Repo {
    fn path(&self) -> Result<PathBuf> {
        data_local_dir()
            .unwrap()
            .join("comtrya")
            .join("plugins")
            .join(self.repo.uri())
            .canonicalize()
            .context("Unable to canonicalize path")
    }
}

impl Source for Repo {
    fn source(&self) -> Result<String> {
        let path = self.path()?;

        let repo = if path.exists() {
            open(&path)?
        } else {
            let url = format!("https://github.com/{}", &self.repo.uri());
            let (mut checkout_result, _) = prepare_clone_bare(url, &path)?
                .with_remote_name("main")?
                .fetch_then_checkout(Discard, &IS_INTERRUPTED)?;
            checkout_result.main_worktree(Discard, &IS_INTERRUPTED)?.0
        };

        let tree = match self.version.as_str() {
            "*" | "" => repo.head_tree()?,
            version => repo
                .find_reference(&format!("tags/{}", version))?
                .peel_to_tree()?,
        };

        match tree.find_entry("plugin.lua") {
            Some(entry) if entry.inner.mode.kind() == EntryKind::Blob => {
                Ok(entry.object()?.data.to_str_lossy().to_string())
            }
            _ => Err(anyhow!("No plugin.lua found")),
        }
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, Default, Eq, PartialOrd, Ord)]
pub struct Dir {
    #[serde(alias = "path")]
    dir: CustomPathBuf,
}

impl Dir {
    fn to_path(&self) -> Result<PathBuf, std::io::Error> {
        self.dir.canonicalize()
    }
}

impl PartialEq for Dir {
    fn eq(&self, other: &Self) -> bool {
        self.to_path().ok() == other.to_path().ok()
    }
}

impl Hash for Dir {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.to_path() {
            Ok(path) => path.hash(state),
            Err(_) => self.dir.hash(state),
        };
    }
}

impl Source for Dir {
    fn source(&self) -> Result<String> {
        read_to_string(self.to_path()?).context("Failed to read file")
    }
}

#[derive(
    JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
#[serde(untagged)]
pub enum RepoOrDir {
    Repo(Repo),
    Dir(Dir),
    #[default]
    Invalid,
}

impl Source for RepoOrDir {
    fn source(&self) -> Result<String> {
        match self {
            RepoOrDir::Repo(repo) => repo.source(),
            RepoOrDir::Dir(dir) => dir.source(),
            RepoOrDir::Invalid => Err(anyhow!("Not a valid source")),
        }
    }
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct TaggedTable {
    #[serde(rename = "$key$")]
    pub tag: String,
    #[serde(flatten)]
    pub table: JsonValue,
}

impl Deref for TaggedTable {
    type Target = JsonValue;

    fn deref(&self) -> &Self::Target {
        &self.table
    }
}

#[serde_as]
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plugin {
    #[serde(flatten)]
    pub source: RepoOrDir,
    #[serde(alias = "options", alias = "spec")]
    #[serde_as(as = "KeyValueMap<_>")]
    pub opts: Vec<TaggedTable>,
}

impl Plugin {
    fn runtime(&self) -> Result<PluginRuntimeSpec> {
        get_plugin(self.source.clone())
    }
}

impl Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plugin: {:?}", self)
    }
}

impl Action for Plugin {
    fn summarize(&self) -> String {
        self.runtime()
            .and_then(|p| p.spec.summary.clone().context(""))
            .unwrap_or("Ran plugin".to_string())
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> Result<Vec<Step>> {
        let runtime = self.runtime()?;
        let planned = self.opts.iter().filter_map(|opt| {
            json_to_lua_value(&opt.table, &runtime.lua)
                .ok()
                .and_then(|v| runtime.plan(&opt.tag, v).and_then(Result::ok))
                .map(|_| opt)
        });

        Ok(planned
            .map(|opt| Step {
                atom: Box::new(PluginExec {
                    plugin_impl: opt.tag.clone(),
                    runtime: runtime.clone(),
                    spec: opt.table.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            })
            .collect::<Vec<_>>())
    }
}
