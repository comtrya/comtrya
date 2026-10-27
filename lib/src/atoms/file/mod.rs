//! Writing, linking, and changing files

/// Permissions
mod chmod;
/// Owner and group, on unix
mod chown;
/// Writing contents
mod contents;
/// Copying
mod copy;
/// Creating empty files
mod create;
/// Writing age-encrypted contents decrypted
mod decrypt;
/// Symlinks
mod link;
/// Removing
mod remove;
/// Unpacking archives
mod unarchive;

use super::Atom;
pub use chmod::Chmod;
pub use chown::Chown;
pub use contents::SetContents;
pub use copy::Copy;
pub use create::Create;
pub use decrypt::Decrypt;
pub use link::Link;
pub use remove::Remove;
pub use unarchive::Unarchive;

/// Atoms that change one file
pub trait FileAtom: Atom {
    /// The file the atom changes. Nothing calls it yet
    fn get_path(&self) -> &std::path::PathBuf;
}
