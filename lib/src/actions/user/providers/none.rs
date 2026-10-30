//! For systems without a known tool: only warns

use super::UserProvider;
use crate::actions::user::{add_group::UserAddGroup, UserVariant};
use crate::actions::Result;
use crate::contexts::Facts;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;

/// Warns that users can't be created or changed on this system
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoneUserProvider {}

impl UserProvider for NoneUserProvider {
    fn add_user(&self, _user: &UserVariant, _facts: &Facts) -> Result<Vec<Step>> {
        warn!("This system does not have a provider for users");
        Ok(vec![])
    }

    fn add_to_group(&self, _user: &UserAddGroup, _facts: &Facts) -> Result<Vec<Step>> {
        warn!(message = "This system does not have a provider for users");
        Ok(vec![])
    }
}
