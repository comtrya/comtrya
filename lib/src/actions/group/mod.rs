pub mod add;
pub mod providers;

use providers::GroupProviders;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Group {
    #[serde(default)]
    provider: GroupProviders,

    #[serde(default)]
    group_name: String,

    #[serde(default)]
    variants: HashMap<os_info::Type, GroupVariant>,
}

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupVariant {
    #[serde(default)]
    provider: GroupProviders,

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
