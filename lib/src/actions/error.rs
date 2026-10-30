//! Why an action can't plan its steps

use crate::atoms::{self, BoxError};
use std::path::PathBuf;
use thiserror::Error as ThisError;

/// Why an action can't plan its steps. The message doesn't repeat the source; print the chain,
/// e.g. with `{:#}` through [`anyhow::Error`]
#[derive(Debug, ThisError)]
pub enum Error {
    /// The action's own `where` can't be evaluated
    #[error("can't evaluate `where: {condition}`")]
    Condition {
        /// The condition
        condition: String,
        /// Why
        #[source]
        source: BoxError,
    },

    /// A field the action can't do without is missing or empty
    #[error("{action} needs `{field}`")]
    Missing {
        /// The action, like "user.add"
        action: &'static str,
        /// The field
        field: &'static str,
    },

    /// A field has a value the action can't use
    #[error("`{field}` {problem}")]
    Invalid {
        /// The field
        field: &'static str,
        /// What's wrong with it, like "must be 64 hex characters, got 'abc'"
        problem: String,
    },

    /// Two fields are set that don't work together
    #[error("{action} can use either `{first}` or `{second}`, not both")]
    Conflict {
        /// The action
        action: &'static str,
        /// One field
        first: &'static str,
        /// The other one
        second: &'static str,
    },

    /// The manifest wasn't read from a directory, so it has no `files` directory
    #[error("the manifest has no directory to find {0} in")]
    NoManifestDir(String),

    /// A file or directory the action reads doesn't exist
    #[error("{} doesn't exist", .0.display())]
    NotFound(PathBuf),

    /// A template can't be rendered
    #[error("can't render {}", path.display())]
    Template {
        /// The file the template is from
        path: PathBuf,
        /// Why
        #[source]
        source: BoxError,
    },

    /// The program of `process_with` can't be started or failed
    #[error("process_with `{command}` failed")]
    ProcessWith {
        /// The command line
        command: String,
        /// Why
        #[source]
        source: atoms::Error,
    },

    /// Neither the action nor `Comtrya.yaml` names a package provider, and the OS has no default
    #[error(
        "there is no default package provider for {os}. Set `provider` on the action, or \
         `package_provider` in Comtrya.yaml"
    )]
    NoPackageProvider {
        /// The OS
        os: String,
    },

    /// The package provider isn't installed, and comtrya can't install it
    #[error("package provider {provider} isn't available")]
    ProviderUnavailable {
        /// The provider
        provider: String,
    },

    /// The package provider can't install packages from local files
    #[error("package provider {provider} can't install local files")]
    NoLocalInstall {
        /// The provider
        provider: String,
    },

    /// An AUR helper refuses to run as root
    #[error(
        "{provider} can't run as root. Run comtrya as a normal user, it asks for sudo where \
         needed, or use `provider: pacman` for packages from the official repositories"
    )]
    AsRoot {
        /// The AUR helper
        provider: String,
    },

    /// The installed packages can't be listed
    #[error("can't list the installed packages with `{command}`")]
    Query {
        /// The command that lists them
        command: String,
        /// Why: it can't be started, or what it wrote to stderr
        #[source]
        source: BoxError,
    },

    /// The GitHub API doesn't answer with the release
    #[error("can't find the release of {repository}")]
    Release {
        /// `owner/repo`
        repository: String,
        /// Why
        #[source]
        source: reqwest::Error,
    },

    /// No asset of the release is for this OS and architecture
    #[error("the release of {repository} has no asset for this OS")]
    NoAsset {
        /// `owner/repo`
        repository: String,
    },

    /// What atoms fail with: reading files, finding and running programs
    #[error(transparent)]
    Atom(#[from] atoms::Error),
}

/// The result of planning an action
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn atom_errors_keep_their_message() {
        let error = Error::from(atoms::Error::CommandNotFound(String::from("dscl")));

        assert_eq!("Command `dscl` not found in path", error.to_string());
    }

    #[test]
    fn the_chain_shows_why() {
        let error = Error::ProcessWith {
            command: String::from("op inject"),
            source: atoms::Error::CommandFailed {
                code: 1,
                stderr: String::from("bad key"),
            },
        };

        assert_eq!(
            "process_with `op inject` failed: Command failed with exit code: 1: bad key",
            format!("{:#}", anyhow::Error::from(error))
        );
    }
}
