//! The tools groups are created with, per OS

// mod freebsd;
// use self::freebsd::FreeBSDUserProvider;
use crate::steps::Step;
mod none;
use self::{freebsd::FreeBSDGroupProvider, none::NoneGroupProvider};
use super::GroupVariant;
use crate::contexts::Contexts;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod freebsd;
mod linux;
use self::linux::LinuxGroupProvider;
mod macos;
use self::macos::MacOsGroupProvider;

/// The tool to create groups with
#[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
pub enum GroupProviders {
    /// For systems comtrya can't create groups on. Only warns
    #[serde(alias = "none")]
    None,

    /// `pw groupadd`
    #[serde(alias = "freebsd")]
    FreeBSD,

    /// `groupadd`
    #[serde(alias = "linux")]
    Linux,

    /// `dscl`
    #[serde(alias = "macos")]
    MacOs,
}

impl GroupProviders {
    /// The implementation of this tool
    pub fn get_provider(self) -> Box<dyn GroupProvider> {
        match self {
            GroupProviders::None => Box::new(NoneGroupProvider {}),
            GroupProviders::FreeBSD => Box::new(FreeBSDGroupProvider {}),
            GroupProviders::Linux => Box::new(LinuxGroupProvider {}),
            GroupProviders::MacOs => Box::new(MacOsGroupProvider {}),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl Default for GroupProviders {
    #[cfg(target_os = "linux")]
    fn default() -> Self {
        GroupProviders::Linux
    }

    #[cfg(not(target_os = "linux"))]
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::FreeBSD => GroupProviders::FreeBSD,
            os_info::Type::Macos => GroupProviders::MacOs,
            _ => GroupProviders::None,
        }
    }
}

/// A tool that creates groups
pub trait GroupProvider {
    /// Steps that create the group. None, with a warning, if the tool or the name is missing
    fn add_group(&self, group: &GroupVariant, contexts: &Contexts) -> Vec<Step>;
}
