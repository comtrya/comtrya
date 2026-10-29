//! `groupadd` on Linux

use super::GroupProvider;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec, utilities};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use which::which;

/// Creates groups with `groupadd`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxGroupProvider {}

impl GroupProvider for LinuxGroupProvider {
    fn add_group(&self, group: &GroupVariant, facts: &Facts) -> anyhow::Result<Vec<Step>> {
        if group.group_name.is_empty() {
            bail!("can't create a group without a group_name");
        }
        let cli = which("groupadd").context("can't find `groupadd` to create groups with")?;

        let privilege_provider = utilities::get_privilege_provider(facts);

        Ok(vec![Step::new(Exec {
            command: cli.display().to_string(),
            arguments: vec![group.group_name.clone()],
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        })])
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod test {
    use crate::actions::group::providers::{GroupProvider, LinuxGroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Facts;

    #[test]
    fn test_add_group() {
        let group_provider = LinuxGroupProvider {};
        let facts = Facts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                group_name: String::from("test"),
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn test_add_group_no_group_name_fails() {
        let group_provider = LinuxGroupProvider {};
        let facts = Facts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                // empty for test purposes
                ..Default::default()
            },
            &facts,
        );

        assert!(steps.is_err());
    }
}
