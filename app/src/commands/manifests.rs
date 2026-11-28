//! `comtrya manifests`: works on manifest files

use super::ComtryaCommand;
use crate::Runtime;
use clap::{Parser, Subcommand};
use comtrya_lib::{error_chain, manifests::upgrade};
use std::path::{Path, PathBuf};

/// The arguments of `comtrya manifests`
#[derive(Parser, Debug)]
pub(crate) struct Manifests {
    /// What to do with the manifests
    #[command(subcommand)]
    command: ManifestsCommand,
}

/// The subcommands of `comtrya manifests`
#[derive(Subcommand, Debug)]
enum ManifestsCommand {
    /// Rewrite version 1 manifests as version 2 (a preview until 0.13.0)
    ///
    /// Prints the upgraded manifest, or with `--write` replaces the file. A manifest is only
    /// upgraded if both versions mean the same with the contexts of this machine; comments
    /// aren't kept.
    #[command(long_about)]
    Upgrade(Upgrade),
}

/// The arguments of `comtrya manifests upgrade`
#[derive(Parser, Debug)]
struct Upgrade {
    /// The manifest files to upgrade
    #[arg(required = true)]
    files: Vec<PathBuf>,

    /// Replace each file with its upgraded version instead of printing it
    #[arg(long)]
    write: bool,
}

impl ComtryaCommand for Manifests {
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        match &self.command {
            ManifestsCommand::Upgrade(upgrade) => upgrade.execute(runtime),
        }
    }
}

impl Upgrade {
    /// Upgrades each file, and fails if any wasn't upgraded. The upgraded manifests go to
    /// stdout, everything else to stderr, so `> new.yaml` gets only the manifest
    fn execute(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let failed = self
            .files
            .iter()
            .filter(|path| !self.upgrade_one(path, runtime))
            .count();
        match failed {
            0 => Ok(()),
            failed => anyhow::bail!(
                "{failed} of {} manifests weren't upgraded",
                self.files.len()
            ),
        }
    }

    /// Upgrades the file at `path`, false if it can't be
    fn upgrade_one(&self, path: &Path, runtime: &Runtime) -> bool {
        let upgraded = match upgrade(path, &runtime.contexts) {
            Ok(upgraded) => upgraded,
            Err(err) => {
                eprintln!("{} isn't upgraded: {}", path.display(), error_chain(&err));
                return false;
            }
        };
        for warning in &upgraded.warnings {
            eprintln!("{}: {warning}", path.display());
        }

        if !self.write {
            if self.files.len() > 1 {
                println!("# {}", path.display());
            }
            print!("{}", upgraded.text);
            return true;
        }
        match std::fs::write(path, &upgraded.text) {
            Ok(()) => {
                eprintln!("Upgraded {}", path.display());
                true
            }
            Err(err) => {
                eprintln!("{} can't be written: {err}", path.display());
                false
            }
        }
    }
}
