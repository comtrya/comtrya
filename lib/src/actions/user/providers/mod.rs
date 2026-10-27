//! The tools users are created and changed with, per OS

mod freebsd;
use self::freebsd::FreeBSDUserProvider;
use crate::steps::Step;
mod none;
use self::none::NoneUserProvider;
use super::{add_group::UserAddGroup, UserVariant};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod linux;
use self::linux::LinuxUserProvider;
mod macos;
use self::macos::MacOSUserProvider;

use crate::contexts::Contexts;

/// The tool to create and change users with
#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum UserProviders {
    /// `pw useradd` and `pw usermod`
    #[serde(alias = "freebsd")]
    FreeBSD,

    /// For systems comtrya can't create users on. Only warns
    #[serde(alias = "none")]
    None,

    /// `useradd` and `usermod`
    #[serde(alias = "linux")]
    Linux,

    /// `dscl`
    #[serde(alias = "macos")]
    MacOs,
}

impl UserProviders {
    /// The implementation of this tool
    pub fn get_provider(self) -> Box<dyn UserProvider> {
        match self {
            UserProviders::FreeBSD => Box::new(FreeBSDUserProvider {}),
            UserProviders::None => Box::new(NoneUserProvider {}),
            UserProviders::Linux => Box::new(LinuxUserProvider {}),
            UserProviders::MacOs => Box::new(MacOSUserProvider {}),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for UserProviders {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        UserProviders::Linux
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            // BSD Operating systems
            os_info::Type::FreeBSD => UserProviders::FreeBSD,
            os_info::Type::Macos => UserProviders::MacOs,
            _ => UserProviders::None,
        }
    }
}

/// A tool that creates and changes users
pub trait UserProvider {
    /// Steps that create the user and add it to its groups
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
    /// Steps that add an existing user to groups
    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>>;
}
