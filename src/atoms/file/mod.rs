mod chmod;
mod chown;
mod create;

use super::Atom;

pub trait FileAtom: Atom {
    fn get_path(&self) -> &std::path::PathBuf;
}
