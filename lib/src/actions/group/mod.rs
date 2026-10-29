//! `group.add`

pub mod add;
pub mod providers;

use providers::GroupProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Creates a group
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    /// How groups are created. Defaults to the tool of this OS
    #[serde(default)]
    provider: GroupProviders,

    /// The name of the group
    #[serde(default)]
    group_name: String,
}

/// The group to create, as the providers take it
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupVariant {
    /// How groups are created
    #[serde(default)]
    provider: GroupProviders,

    /// The name of the group
    #[serde(default)]
    group_name: String,
}

impl From<&Group> for GroupVariant {
    fn from(group: &Group) -> Self {
        GroupVariant {
            provider: group.provider.clone(),
            group_name: group.group_name.clone(),
        }
    }
}
