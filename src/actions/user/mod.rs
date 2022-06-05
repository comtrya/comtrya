pub mod add;
pub mod userproviders;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;
use userproviders::UserProviders;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
    variants: HashMap<os_info::Type, UserVariant>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
}

impl UserVariant {
    fn users(&self) -> Vec<String> {
        let string: Vec<String> = vec![];
        string
    }
}

impl From<&User> for UserVariant {
    fn from(user: &User) -> Self {
        let os = os_info::get();

        // Check for variant configuration for this OS
        let variant = user.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return UserVariant {
                provider: user.provider.clone(),
                username: user.username.clone(),
                home_dir: user.home_dir.clone(),
                fullname: user.fullname.clone(),
                shell: user.shell.clone(),
            };
        };

        let variant = variant.unwrap();

        debug!(message = "Built Variant", variant = ?variant);

        let mut user = UserVariant {
            provider: user.provider.clone(),
            username: user.username.clone(),
            home_dir: user.home_dir.clone(),
            fullname: user.fullname.clone(),
            shell: user.shell.clone(),
        };

        user.provider = variant.provider.clone();

        user
    }
}