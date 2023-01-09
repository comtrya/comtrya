use super::UserProvider;
use crate::actions::user::{add_group::UserAddGroup, UserVariant};
use crate::atoms::command::Exec;
use crate::steps::Step;
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSUserProvider {}

impl UserProvider for MacOSUserProvider {
    fn add_user(&self, user: &UserVariant) -> anyhow::Result<Vec<Step>> {
        let mut args: Vec<String> = vec![];
        let cli = match which("dscl") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not find proper user add tool");
                return Ok(vec![]);
            }
        };

        // is a user name isn't provided, cant create a new user
        if user.username.is_empty() {
            warn!("Unable to create user without a username");
            return Ok(vec![]);
        }

        let mut username = String::from("/Users/");
        username.push_str(user.username.clone().as_str());

        args.push(String::from("."));
        args.push(String::from("-create"));
        args.push(username);

        if !user.shell.is_empty() {
            args.push(String::from("UserShell"));
            args.push(user.shell.clone());
        }

        if !user.fullname.is_empty() {
            args.push(String::from("RealName"));
            args.push(String::from("\"{user.fullename}\""));
        }

        let steps: Vec<Step> = vec![Step {
            atom: Box::new(Exec {
                command: cli.display().to_string(),
                arguments: vec![].into_iter().chain(args.clone()).collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        Ok(steps)
    }

    fn add_to_group(&self, _user: &UserAddGroup) -> anyhow::Result<Vec<Step>> {
        warn!(message = "Adding users to group not implemented for platform");
        Ok(vec![])
    }
}

#[cfg(target_os = "macos")]
#[cfg(test)]
mod test {
    use crate::actions::user::providers::{MacOSUserProvider, UserProvider};
    use crate::actions::user::UserVariant;

    #[test]
    fn test_add_user() {
        let user_provider = MacOSUserProvider {};
        let steps = user_provider.add_user(&UserVariant {
            username: String::from("test"),
            shell: String::from("sh"),
            home_dir: String::from("/home/test"),
            fullname: String::from("Test User"),
            group: vec![],
            ..Default::default()
        });

        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn test_add_user_no_username() {
        let user_provider = MacOSUserProvider {};
        let steps = user_provider.add_user(&UserVariant {
            username: String::from(""),
            shell: String::from("sh"),
            home_dir: String::from("/home/test"),
            fullname: String::from("Test User"),
            group: vec![],
            ..Default::default()
        });

        assert_eq!(steps.unwrap().len(), 0);
    }
}
