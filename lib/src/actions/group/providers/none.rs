use super::GroupProvider;
use crate::actions::group::GroupVariant;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoneGroupProvider {}

impl GroupProvider for NoneGroupProvider {
    fn add_group(&self, _group: &GroupVariant) -> Vec<Step> {
        warn!("This system does not have a provider for groups");
        vec![]
    }
}
