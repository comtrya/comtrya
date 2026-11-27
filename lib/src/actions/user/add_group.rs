//! `user.group`: adds an existing user to groups

use super::providers::UserProviders;
use crate::actions::{Action, PlanCx, Result};
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

// pub type UserAddGroup = User;
/// Adds a user to groups
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct UserAddGroup {
    /// The user
    #[serde(default)]
    pub username: String,

    /// The groups to add the user to
    #[serde(default)]
    pub group: Vec<String>,

    /// How users are changed. Defaults to the tool of this OS
    #[serde(default)]
    pub provider: UserProviders,
}

impl Action for UserAddGroup {
    fn summarize(&self) -> String {
        let groups = if self.group.len() == 1 {
            "group"
        } else {
            "groups"
        };
        format!(
            "Add user {} to {groups} {}",
            self.username,
            self.group.join(", ")
        )
    }

    fn plan(&self, _manifest: &Manifest, cx: &PlanCx) -> Result<Vec<Step>> {
        let box_provider = self.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        atoms.append(&mut provider.add_to_group(self, cx.facts)?);

        Ok(atoms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_names_the_groups() {
        let mut action = UserAddGroup {
            username: String::from("ben"),
            group: vec![String::from("wheel")],
            provider: UserProviders::default(),
        };
        assert_eq!("Add user ben to group wheel", action.summarize());

        action.group.push(String::from("docker"));
        assert_eq!("Add user ben to groups wheel, docker", action.summarize());
    }
}
