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

        // Check for variant configuration for this OS
        let variant = group.variants.get(&os.os_type());

        // No variant overlays
        if variant.is_none() {
            return GroupVariant {
                provider: group.provider.clone(),
                group_name: group.group_name.clone(),
            };
        };

        let variant = variant.unwrap();

        debug!(message = "Built Variant", variant = ?variant);

        let mut group = GroupVariant {
            provider: group.provider.clone(),
            group_name: group.group_name.clone(),
        };

        group.provider = variant.provider.clone();

        group
    }
}
