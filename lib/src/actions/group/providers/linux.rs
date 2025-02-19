use super::GroupProvider;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::group::GroupVariant, atoms::command::Exec, utilities};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxGroupProvider {}

impl GroupProvider for LinuxGroupProvider {
    fn add_group(&self, group: &GroupVariant, contexts: &Contexts) -> Vec<Step> {
        let cli = match which("groupadd") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper group add tool");
                return vec![];
            }
        };

        if group.group_name.is_empty() {
            warn!(message = "Unable to create group without a group name");
            return vec![];
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
                arguments: vec![group.group_name.clone()],
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod test {
    use crate::actions::group::providers::{GroupProvider, LinuxGroupProvider};
    use crate::actions::group::GroupVariant;
    use crate::contexts::Contexts;

    #[test]
    fn test_add_group() {
        let group_provider = LinuxGroupProvider {};
        let contexts = Contexts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                group_name: String::from("test"),
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_add_group_no_group_name() {
        let group_provider = LinuxGroupProvider {};
        let contexts = Contexts::default();
        let steps = group_provider.add_group(
            &GroupVariant {
                // empty for test purposes
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.len(), 0);
    }
}
