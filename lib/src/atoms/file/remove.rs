use std::path::PathBuf;

use crate::atoms::Atom;

use super::FileAtom;

pub struct Remove {
    pub target: PathBuf,
}

impl FileAtom for Remove {
    fn get_path(&self) -> &std::path::PathBuf {
        &self.target
    }
}

impl std::fmt::Display for Remove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Atom for Remove {
    fn plan(&self) -> anyhow::Result<crate::atoms::Outcome> {
        todo!()
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        todo!()
    }
}
