pub mod add;
pub mod add_group;
pub mod providers;

use providers::UserProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct User {
    #[serde(default)]
    provider: UserProviders,

    #[serde(default)]
    username: String,

    #[serde(default)]
    home_dir: String,

    #[serde(default)]
    fullname: String,

    #[serde(default)]
    shell: String,

    #[serde(default)]
    group: Vec<String>,

    #[serde(default)]
    variants: HashMap<os_info::Type, UserVariant>,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserVariant {
    #[serde(default)]
    provider: UserProviders,

    #[serde(default)]
    username: String,

    #[serde(default)]
    home_dir: String,

    #[serde(default)]
    fullname: String,

    #[serde(default)]
    shell: String,

    #[serde(default)]
    group: Vec<String>,
}

impl From<&User> for UserVariant {
    fn from(user: &User) -> Self {
        let user = user.clone();
        let mut user_variant = UserVariant {
            provider: user.provider,
            username: user.username,
            home_dir: user.home_dir,
            fullname: user.fullname,
            shell: user.shell,
            group: user.group,
        };

        let Some(variant) = user.variants.get(&os_info::get().os_type()) else {
            return user_variant;
        };

        debug!(message = "Built Variant", variant = ?variant);

        user_variant.provider = variant.provider.clone();
        user_variant
    }
}
