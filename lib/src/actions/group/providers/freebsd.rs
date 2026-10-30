//! `pw groupadd` on FreeBSD

use super::GroupProvider;
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec};
use anyhow::bail;
use serde::{Deserialize, Serialize};

/// Creates groups with `pw groupadd`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeBSDGroupProvider {}

impl GroupProvider for FreeBSDGroupProvider {
    fn add_group(&self, group: &GroupVariant, _facts: &Facts) -> anyhow::Result<Vec<Step>> {
        if group.group_name.is_empty() {
            bail!("can't create a group without a group_name");
        }

        Ok(vec![Step::new(Exec {
            command: String::from("/usr/bin/pw"),
            arguments: vec![String::from("groupadd"), group.group_name.clone()],
            ..Default::default()
        })
        .as_root()])
    }
}

#[cfg(target_os = "freebsd")]
#[cfg(test)]
mod test {
    use crate::actions::group::providers::{FreeBSDGroupProvider, GroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Facts;

    #[test]
    fn test_add_group() {
        let group_provider = FreeBSDGroupProvider {};
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
        let group_provider = FreeBSDGroupProvider {};
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
