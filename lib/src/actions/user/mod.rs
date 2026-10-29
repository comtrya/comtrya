//! `user.add` and `user.group`

pub mod add;
pub mod add_group;
pub mod providers;

use providers::UserProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Creates a user, unless it exists already
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct User {
    /// How users are created. Defaults to the tool of this OS
    #[serde(default)]
    provider: UserProviders,

    /// The name of the user. Nothing happens without it
    #[serde(default)]
    username: String,

    /// The home directory, created if it's missing. The tool's default without it
    #[serde(default)]
    home_dir: String,

    /// The full name, as the comment of the user
    #[serde(default)]
    fullname: String,

    /// The login shell. The tool's default without it
    #[serde(default)]
    shell: String,

    /// Groups to add the user to
    #[serde(default)]
    group: Vec<String>,
}

/// The user to create, as the providers take it
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserVariant {
    /// How users are created
    #[serde(default)]
    provider: UserProviders,

    /// The name of the user
    #[serde(default)]
    username: String,

    /// The home directory, created if it's missing
    #[serde(default)]
    home_dir: String,

    /// The full name, as the comment of the user
    #[serde(default)]
    fullname: String,

    /// The login shell
    #[serde(default)]
    shell: String,

    /// Groups to add the user to
    #[serde(default)]
    group: Vec<String>,
}

impl From<&User> for UserVariant {
    fn from(user: &User) -> Self {
        UserVariant {
            provider: user.provider.clone(),
            username: user.username.clone(),
            home_dir: user.home_dir.clone(),
            fullname: user.fullname.clone(),
            shell: user.shell.clone(),
            group: user.group.clone(),
        }
    }
}
