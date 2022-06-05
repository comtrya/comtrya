pub mod add;
pub mod userproviders;

use userproviders::UserProviders;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

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
    uid: String,

    #[serde(default)]
    gid: String,

    #[serde(default)]
    password: String,

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

    #[serde(default)]
    uid: String,

    #[serde(default)]
    gid: String,

    #[serde(default)]
    password: String,

    // #[serde(default)]
    // extra_args: Vec<String>,
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
		uid: user.uid.clone(),
		gid: user.gid.clone(),
		password: user.password.clone(),
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
	    uid: user.uid.clone(),
	    gid: user.gid.clone(),
	    password: user.password.clone(),
        };

        // if variant.name.is_some() {
        //     user.name = variant.name.clone();
        // }

        // if !variant.list.is_empty() {
        //     user.list = variant.list.clone();
        // }

        // if variant.repository.is_some() {
        //     user.repository = variant.repository.clone();
        // };

        // if variant.key.is_some() {
        //     user.key = variant.key.clone();
        // }

        // I've been torn about this, but here's my logic.
        // Variants, when being used, shouldn't use the provider
        // of the main definition; as we're not the core OS.
        // Even if the omission of a provider for a variant gets us
        // the default, that's most likely still expected behaviour.
        // Right?
        user.provider = variant.provider.clone();

        user
    }
}
