use super::GroupProvider;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[cfg(tests)]
mod test {
    use crate::actions::group::*;

    #[test]
    fn test_add_group() {
        let group_provider = FreeBSDGroupProvider {};
        let steps = user_provider.add_user(&GroupVariant { group_name: test });

        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_add_group_no_group_name() {
        let group_provider = FreeBSDGroupProvider {};
        let steps = group_provider.add_user(&GroupVariant {
            // empty for test purposes
        });

        assert_eq!(steps.len(), 0);
    }
}