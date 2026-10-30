//! Why an atom can't check or make its change

use std::path::{Path, PathBuf};
use thiserror::Error as ThisError;

/// An error from a library whose types don't matter to callers, like gix or age
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Why an atom can't check or make its change. The message doesn't repeat the source; print the
/// chain, e.g. with `{:#}` through [`anyhow::Error`]
#[derive(Debug, ThisError)]
pub enum Error {
    /// Reading or changing the file system failed
    #[error("can't {doing} {}", path.display())]
    Io {
        /// What failed, like "read" or "create the directory"
        doing: &'static str,
        /// The path it failed on
        path: PathBuf,
        /// Why
        #[source]
        source: std::io::Error,
    },

    /// A path is something else than the atom needs or may replace
    #[error("{} isn't a {kind}", path.display())]
    NotA {
        /// The path
        path: PathBuf,
        /// What it should be: "file", "directory" or "link"
        kind: &'static str,
    },

    /// A directory to remove still has entries
    #[error("{} isn't empty", path.display())]
    NotEmpty {
        /// The directory
        path: PathBuf,
    },

    /// The directory of a file to remove is read-only
    #[error("no permission to remove {}", path.display())]
    ReadOnly {
        /// The file
        path: PathBuf,
    },

    /// A user name that doesn't exist on this system
    #[error("user {0} doesn't exist")]
    UnknownUser(String),

    /// A group name that doesn't exist on this system
    #[error("group {0} doesn't exist")]
    UnknownGroup(String),

    /// Copying a directory into itself would never end
    #[error("cannot copy the directory {} into itself ({})", from.display(), to.display())]
    CopyIntoItself {
        /// The directory to copy
        from: PathBuf,
        /// Where it would go, inside `from`
        to: PathBuf,
    },

    /// Encrypted contents can't be decrypted, e.g. with a wrong passphrase
    #[error("can't decrypt {}", path.display())]
    Decrypt {
        /// The file the contents are for
        path: PathBuf,
        /// Why
        #[source]
        source: BoxError,
    },

    /// A download failed, or the server answered with an error
    #[error("can't download {url}")]
    Download {
        /// The URL
        url: String,
        /// Why
        #[source]
        source: reqwest::Error,
    },

    /// A download isn't what `sha256` says
    #[error("SHA-256 of {url} doesn't match: expected {expected}, got {actual}")]
    Checksum {
        /// The URL
        url: String,
        /// The hash from the manifest
        expected: String,
        /// The hash of what was downloaded
        actual: String,
    },

    /// Cloning or updating a repository failed
    #[error("can't {doing} {repository}")]
    Git {
        /// What failed, like "clone"
        doing: &'static str,
        /// The repository URL
        repository: String,
        /// Why
        #[source]
        source: BoxError,
    },

    /// A program isn't installed, or not in the `PATH` it's looked up in
    #[error("Command `{0}` not found in path")]
    CommandNotFound(String),

    /// A program can't be started
    #[error("can't start {command}")]
    Spawn {
        /// The program
        command: String,
        /// Why
        #[source]
        source: std::io::Error,
    },

    /// A program exited with an error
    #[error("Command failed with exit code: {code}{}", stderr_suffix(stderr))]
    CommandFailed {
        /// The exit code, 1 if a signal ended the program
        code: i32,
        /// What it wrote to stderr, trimmed
        stderr: String,
    },

    /// A signal ended a program, like `SIGKILL` from the OOM killer
    #[error("Command was killed by signal {signal}{}", stderr_suffix(stderr))]
    Killed {
        /// The signal number
        signal: i32,
        /// What it wrote to stderr until then, trimmed
        stderr: String,
    },

    /// Only programs can run as root until the privileged helper (#502) runs any atom
    #[error("{atom}: only programs can run as root for now")]
    NotElevatable {
        /// The atom, as it's shown
        atom: String,
    },
}

/// The result of an atom
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// For `map_err`: an [`Error::Io`] doing `doing` on `path`
    pub(crate) fn io(doing: &'static str, path: &Path) -> impl FnOnce(std::io::Error) -> Error {
        let path = path.to_path_buf();
        move |source| Error::Io {
            doing,
            path,
            source,
        }
    }
}

/// `: <stderr>` for a program that wrote to stderr, nothing otherwise
fn stderr_suffix(stderr: &str) -> String {
    match stderr {
        "" => String::new(),
        stderr => format!(": {stderr}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn an_io_error_names_the_path_and_keeps_the_cause() {
        let error = Error::io("read", Path::new("/etc/x"))(std::io::ErrorKind::NotFound.into());

        assert_eq!("can't read /etc/x", error.to_string());
        assert_eq!(
            "can't read /etc/x: entity not found",
            format!("{:#}", anyhow::Error::from(error))
        );
    }
}
