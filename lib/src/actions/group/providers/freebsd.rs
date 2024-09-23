use super::GroupProvider;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeBSDGroupProvider {}

impl GroupProvider for FreeBSDGroupProvider {
    fn add_group(&self, group: &GroupVariant) -> Vec<Step> {
        if group.group_name.is_empty() {
            warn!(message = "Unable to create group without a group name");
            return vec![];
        }

        vec![Step {
            atom: Box::new(Exec {
                command: String::from("/usr/bin/pw"),
                arguments: vec![String::from("groupadd"), group.group_name.clone()],
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}

#[cfg(target_os = "freebsd")]
#[cfg(test)]
mod test {
    use crate::actions::group::providers::{FreeBSDGroupProvider, GroupProvider};
    use crate::actions::group::GroupVariant;

    #[test]
    fn test_add_group() {
        let group_provider = FreeBSDGroupProvider {};
        let steps = group_provider.add_group(&GroupVariant {
            group_name: String::from("test"),
            ..Default::default()
        });

        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_add_group_no_group_name() {
        let group_provider = FreeBSDGroupProvider {};
        let steps = group_provider.add_group(&GroupVariant {
            // empty for test purposes
            ..Default::default()
        });

        assert_eq!(steps.len(), 0);
    }
}
