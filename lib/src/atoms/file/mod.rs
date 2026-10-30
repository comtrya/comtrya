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

pub use chmod::Chmod;
pub use chown::Chown;
pub use contents::SetContents;
pub use copy::Copy;
pub use create::Create;
pub use decrypt::Decrypt;
pub use link::Link;
pub use remove::Remove;
pub use unarchive::Unarchive;
