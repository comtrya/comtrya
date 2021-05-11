mod chmod;
mod chown;
mod copy;
mod create;

use super::Atom;

pub trait FileAtom: Atom {
    // Don't think this is needed? Validate soon
    fn get_path(&self) -> &std::path::PathBuf;
}
