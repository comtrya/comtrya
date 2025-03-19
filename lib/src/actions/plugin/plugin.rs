use std::{
    fmt::{self, Display, Formatter},
    fs,
    hash::{Hash, Hasher},
    ops::Deref,
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use dirs_next::data_local_dir;
use gix::{
    bstr::ByteSlice, diff::object::tree::EntryKind, interrupt::IS_INTERRUPTED, open,
    prepare_clone_bare, progress::Discard,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_with::{serde_as, KeyValueMap};

use super::{get_plugin, PluginRuntimeSpec};
use crate::{
    actions::Action, atoms::plugin::PluginExec, contexts::Contexts, manifests::Manifest,
    steps::Step, utilities::lua::json_to_lua_value, utilities::CustomPathBuf,
};

#[derive(
    JsonSchema, Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct RepoUri {
    pub username: String,
    pub repo: String,
}

impl Display for RepoUri {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.username, self.repo)
    }
}

impl TryFrom<String> for RepoUri {
    type Error = anyhow::Error;
    fn try_from(s: String) -> Result<Self> {
        match s.split_once('/') {
            Some((username, repo)) => Ok(Self {
                username: username.to_string(),
                repo: repo.to_string(),
            }),
            _ => Err(anyhow!("repo must be in format 'username/repo'")),
        }
    }
}

pub trait Source {
    fn source(&self) -> Result<String>;
}

#[derive(
    JsonSchema, Default, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(untagged)]
pub enum Version {
    #[default]
    Stable,
    Latest,
    Tagged(String),
}

impl From<String> for Version {
    fn from(s: String) -> Self {
        match s.as_str() {
            "" | "stable" => Self::Stable,
            "*" | "latest" => Self::Latest,
            _ => Self::Tagged(s),
        }
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, Default, Eq, PartialOrd, Ord)]
pub struct Repo {
    #[serde(alias = "repository")]
    repo: RepoUri,

    #[serde(alias = "tag", default)]
    version: Version,
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
            self.repo.to_string().hash(state);
            self.version.hash(state);
        }
    }
}

impl Repo {
    fn path(&self) -> Result<PathBuf> {
        let path = data_local_dir().context("Failed to locate local data directory")?;

        path.join("comtrya")
            .join("plugins")
            .join(self.repo.to_string())
            .canonicalize()
            .map_err(anyhow::Error::from)
    }
}

impl Source for Repo {
    fn source(&self) -> Result<String> {
        let path = self.path()?;

        let repo = if !path.exists() {
            let url = format!("https://github.com/{}", self.repo);
            let (mut checkout_result, _) = prepare_clone_bare(url, &path)?
                .with_remote_name("main")?
                .fetch_then_checkout(Discard, &IS_INTERRUPTED)?;
            checkout_result.main_worktree(Discard, &IS_INTERRUPTED)?.0
        } else {
            open(&path)?
        };

        let tree = match self.version {
            Version::Stable => repo.find_reference("tags/latest")?.peel_to_tree()?,
            Version::Latest => repo.head_tree()?,
            Version::Tagged(ref version) => repo
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

impl Deref for Dir {
    type Target = CustomPathBuf;

    fn deref(&self) -> &Self::Target {
        &self.dir
    }
}

impl PartialEq for Dir {
    fn eq(&self, other: &Self) -> bool {
        self.canonicalize().ok() == other.canonicalize().ok()
    }
}

impl Hash for Dir {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Ok(path) = self.canonicalize() {
            path.hash(state)
        } else {
            self.dir.hash(state)
        };
    }
}

impl Source for Dir {
    fn source(&self) -> Result<String> {
        fs::read_to_string(self.canonicalize()?).context("Failed to read file")
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
        get_plugin(self.source.clone()).map_err(anyhow::Error::from)
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
                .and_then(|v| runtime.plan_action(&opt.tag, v).and_then(Result::ok))
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
