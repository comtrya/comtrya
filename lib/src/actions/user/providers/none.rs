use super::UserProvider;
use crate::actions::user::UserVariant;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoneUserProvider {}

impl UserProvider for NoneUserProvider {
    fn add_user(&self, _user: &UserVariant) -> Vec<Step> {
        warn!("This system does not have a provider for users");
        vec![]
    }
}
