//! `group.add`

pub mod add;
pub mod providers;

use providers::GroupProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

/// Creates a group
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    /// How groups are created. Defaults to the tool of this OS
    #[serde(default)]
    provider: GroupProviders,

    /// The name of the group
    #[serde(default)]
    group_name: String,

    /// Settings for an OS. Only the `provider` of a variant is used
    #[serde(default)]
    variants: HashMap<os_info::Type, GroupVariant>,
}

/// The group to create on this OS: the action, with the provider of the variant for this OS
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupVariant {
    /// How groups are created on this OS
    #[serde(default)]
    provider: GroupProviders,

    /// Not used: the action's `group_name` applies
    #[serde(default)]
    group_name: String,
}

impl From<&Group> for GroupVariant {
    fn from(group: &Group) -> Self {
        let os = os_info::get();

        // The variant for this OS overlays the action, if there is one
        let Some(variant) = group.variants.get(&os.os_type()) else {
            return GroupVariant {
                provider: group.provider.clone(),
                group_name: group.group_name.clone(),
            };
        };

        debug!(message = "Built Variant", variant = ?variant);

        let mut group = GroupVariant {
            provider: group.provider.clone(),
            group_name: group.group_name.clone(),
        };

        group.provider = variant.provider.clone();

        group
    }
}
