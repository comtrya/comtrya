//! Clones a repository

use super::super::Change;
use crate::atoms::Outcome;
use gix::interrupt;
use gix::remote::fetch::Shallow;
use gix::{progress::Discard, Url};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Clones a repository, unless `directory` exists
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Clone {
    /// The URL of the repository
    #[serde(with = "super::url_as_string")]
    pub repository: Url,
    /// Where to clone it to
    pub directory: PathBuf,
    /// Branch or tag to check out, the remote's `HEAD` if `None`
    pub reference: Option<String>,
}

impl std::fmt::Display for Clone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.reference {
            Some(reference) => write!(
                f,
                "GitClone {}#{reference} to {}",
                self.repository,
                self.directory.display()
            ),
            None => write!(
                f,
                "GitClone {} to {}",
                self.repository,
                self.directory.display()
            ),
        }
    }
}

impl Change for Clone {
    #[instrument(name = "git.clone.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: !self.directory.exists(),
        })
    }

    #[instrument(name = "git.clone.execute", level = "info", skip(self))]
    fn execute(&self) -> anyhow::Result<()> {
        clone_repository(
            &self.repository,
            &self.directory,
            self.reference.as_deref(),
            false,
        )
    }
}

/// Clones `repository` into `directory`, with only the latest commit if `shallow`
pub(crate) fn clone_repository(
    repository: &Url,
    directory: &Path,
    reference: Option<&str>,
    shallow: bool,
) -> anyhow::Result<()> {
    unsafe {
        interrupt::init_handler(1, || {})?;
    };

    std::fs::create_dir_all(directory)?;

    let mut prepare_clone =
        gix::prepare_clone(repository.clone(), directory)?.with_ref_name(reference)?;
    if shallow {
        prepare_clone = prepare_clone.with_shallow(Shallow::DepthAtRemote(NonZeroU32::MIN));
    }
    let (mut prepare_checkout, _) =
        prepare_clone.fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED)?;

    let (repo, _) = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED)?;

    let _ = repo
        .find_default_remote(gix::remote::Direction::Fetch)
        .expect("always present after clone")?;

    Ok(())
}
