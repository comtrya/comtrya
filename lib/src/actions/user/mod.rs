//! `user.add` and `user.group`

pub mod add;
pub mod add_group;
pub mod providers;

use providers::UserProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

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

    /// Settings for an OS. Only the `provider` of a variant is used
    #[serde(default)]
    variants: HashMap<os_info::Type, UserVariant>,
}

/// The user to create on this OS: the action, with the provider of the variant for this OS
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserVariant {
    /// How users are created on this OS
    #[serde(default)]
    provider: UserProviders,

    /// Not used: the action's field applies
    #[serde(default)]
    username: String,

    /// Not used: the action's field applies
    #[serde(default)]
    home_dir: String,

    /// Not used: the action's field applies
    #[serde(default)]
    fullname: String,

    /// Not used: the action's field applies
    #[serde(default)]
    shell: String,

    /// Not used: the action's field applies
    #[serde(default)]
    group: Vec<String>,
}

impl From<&User> for UserVariant {
    fn from(user: &User) -> Self {
        let os = os_info::get();

        // The variant for this OS overlays the action, if there is one
        let Some(variant) = user.variants.get(&os.os_type()) else {
            return UserVariant {
                provider: user.provider.clone(),
                username: user.username.clone(),
                home_dir: user.home_dir.clone(),
                fullname: user.fullname.clone(),
                shell: user.shell.clone(),
                group: user.group.clone(),
            };
        };

        debug!(message = "Built Variant", variant = ?variant);

        let mut user = UserVariant {
            provider: user.provider.clone(),
            username: user.username.clone(),
            home_dir: user.home_dir.clone(),
            fullname: user.fullname.clone(),
            shell: user.shell.clone(),
            group: user.group.clone(),
        };

        user.provider = variant.provider.clone();

        user
    }
}
