use super::UserProvider;
use crate::actions::user::{add_group::UserAddGroup, UserVariant};
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoneUserProvider {}

impl UserProvider for NoneUserProvider {
    fn add_user(&self, _user: &UserVariant) -> anyhow::Result<Vec<Step>> {
        warn!("This system does not have a provider for users");
        Ok(vec![])
    }

    fn add_to_group(&self, _user: &UserAddGroup) -> anyhow::Result<Vec<Step>> {
        warn!(message = "This system does not have a provider for users");
        Ok(vec![])
    }
}
