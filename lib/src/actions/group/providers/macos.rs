//! `dscl` on macOS

use super::GroupProvider;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec, utilities};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use which::which;

/// Creates groups with `dscl`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOsGroupProvider {}

impl GroupProvider for MacOsGroupProvider {
    fn add_group(&self, group: &GroupVariant, facts: &Facts) -> anyhow::Result<Vec<Step>> {
        if group.group_name.is_empty() {
            bail!("can't create a group without a group_name");
        }
        let cli = which("dscl").context("can't find `dscl` to create groups with")?;

        let mut group_creation_string = String::from("/Groups/");
        group_creation_string.push_str(group.group_name.clone().as_str());

        let args: Vec<String> = vec![
            ".".to_string(),
            "create".to_string(),
            group_creation_string.to_owned(),
        ];

        let privilege_provider = utilities::get_privilege_provider(facts);

        Ok(vec![Step::new(Exec {
            command: cli.display().to_string(),
            arguments: args.into_iter().collect(),
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        })])
    }
}

#[cfg(target_os = "macos")]
#[cfg(test)]
mod test {
    use crate::actions::group::providers::{GroupProvider, MacOsGroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Facts;

    #[test]
    fn test_add_group() {
        let group_provider = MacOsGroupProvider {};
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
        let group_provider = MacOsGroupProvider {};
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
