mod freebsd;
use self::freebsd::FreeBSDUserProvider;
use crate::steps::Step;
mod none;
use self::none::NoneUserProvider;
mod linux;
use self::linux::LinuxUserProvider;
use super::UserVariant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum UserProviders {
    #[serde(alias = "freebsd")]
    FreeBSDUserProvider,

    #[serde(alias = "none")]
    NoneUserProvider,

    #[serde(alias = "linux")]
    LinuxUserProvider,
}

impl UserProviders {
    pub fn get_provider(self) -> Box<dyn UserProvider> {
        match self {
            UserProviders::FreeBSDUserProvider => Box::new(FreeBSDUserProvider {}),
            UserProviders::NoneUserProvider => Box::new(NoneUserProvider {}),
	    UserProviders::LinuxUserProvider => Box::new(LinuxUserProvider {}),
        }
    }
}

impl Default for UserProviders {
    fn default() -> Self {
        let info = os_info::get();

        #[cfg(any(target_os = "linux"))]
        return UserProviders::LinuxUserProvider;

        match info.os_type() {
            // BSD Operating systems
            os_info::Type::FreeBSD => UserProviders::FreeBSDUserProvider,
            _ => UserProviders::NoneUserProvider,
        }
    }
}

pub trait UserProvider {
    fn add_user(&self, user: &UserVariant) -> Vec<Step>;
}
