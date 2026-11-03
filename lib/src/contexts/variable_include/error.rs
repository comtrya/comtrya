//! Why the variables of an `include_variables` source can't be read

use crate::atoms::BoxError;
use std::path::PathBuf;
use thiserror::Error as ThisError;

/// Why the variables of an `include_variables` source can't be read. The message doesn't repeat
/// the source, see [`crate::error_chain`]. A 1Password item's values are secrets, so
/// no variant holds them
#[derive(Debug, ThisError)]
pub enum Error {
    /// The source isn't a URL
    #[error("{url} isn't a valid URL")]
    Url {
        /// The source
        url: String,
        /// Why
        #[source]
        source: BoxError,
    },

    /// The URL has a scheme comtrya doesn't read
    #[error("Unknown variable include scheme: {0}")]
    UnknownScheme(String),

    /// A file can't be read
    #[error("can't read {}", path.display())]
    Read {
        /// The file
        path: PathBuf,
        /// Why
        #[source]
        source: std::io::Error,
    },

    /// A file isn't valid YAML or TOML, or has no keys at the top
    #[error("can't parse {}", path.display())]
    Parse {
        /// The file
        path: PathBuf,
        /// Why
        #[source]
        source: BoxError,
    },

    /// A `dns+txt://` URL without a host
    #[error("{0} has no host")]
    NoHost(String),

    /// The TXT records of a host can't be looked up
    #[error("can't look up the TXT records of {host}")]
    Dns {
        /// The host
        host: String,
        /// Why
        #[source]
        source: BoxError,
    },

    /// An `op://` source that isn't `op://<vault>/<item>[?account=<account>]`
    #[error("{0} is not of the form op://<vault>/<item>[?account=<account>]")]
    OpUrl(String),

    /// The 1Password CLI isn't installed
    #[error(
        "{0} needs the 1Password CLI `op`, see https://developer.1password.com/docs/cli/get-started/"
    )]
    OpMissing(String),

    /// The 1Password CLI can't be started
    #[error("failed to run `op` for {include}")]
    OpRun {
        /// The source
        include: String,
        /// Why
        #[source]
        source: std::io::Error,
    },

    /// The 1Password CLI fails, usually because it isn't signed in
    #[error(
        "`op` failed to read {include}: {stderr}. Sign in with `op signin`, or turn on the \
         1Password app integration"
    )]
    OpFailed {
        /// The source
        include: String,
        /// What `op` wrote to stderr, trimmed
        stderr: String,
    },

    /// The 1Password CLI answers with something that isn't an item. The parser's error isn't
    /// kept, because it can quote the secrets
    #[error("`op` returned an item for {0} that can't be read")]
    OpItem(String),
}

/// The result of reading a source
pub type Result<T> = std::result::Result<T, Error>;
