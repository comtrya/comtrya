mod freebsd;
use self::freebsd::FreeBSDUserProvider;
use crate::steps::Step;
mod none;
use self::none::NoneUserProvider;
use super::UserVariant;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum UserProviders {
    #[serde(alias = "freebsd")]
    FreeBSDUserProvider,

    #[serde(alias = "none")]
    NoneUserProvider,
}

impl UserProviders {
    pub fn get_provider(self) -> Box<dyn UserProvider> {
        match self {
            UserProviders::FreeBSDUserProvider => Box::new(FreeBSDUserProvider {}),
            UserProviders::NoneUserProvider => Box::new(NoneUserProvider {}),
        }
    }
}

impl Default for UserProviders {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            // BSD Operating systems
            os_info::Type::FreeBSD => UserProviders::FreeBSDUserProvider,
            _ => UserProviders::NoneUserProvider,
            //     _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}

pub trait UserProvider {
    fn add_user(&self, user: &UserVariant) -> Vec<Step>;
}
