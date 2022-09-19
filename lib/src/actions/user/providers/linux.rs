use super::UserProvider;
use crate::steps::Step;
use crate::{actions::user::UserVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};
use tracing::warn;
use which::which;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinuxUserProvider {}

impl UserProvider for LinuxUserProvider {
    fn add_user(&self, user: &UserVariant) -> Vec<Step> {
        let mut args: Vec<String> = vec![];
        let cli = match which("useradd") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper user add tool");
                return vec![];
            }
        };

        // is a user name isn't provided, cant create a new user
        if user.username.is_empty() {
            warn!(message = "Unable to create user without a username");
            return vec![];
        }

        args.push(user.username.clone());

        if !user.home_dir.is_empty() {
            args.push(String::from("-m"));
            args.push(String::from("-d"));
            args.push(user.home_dir.clone());
        }

        if !user.shell.is_empty() {
            args.push(String::from("-s"));
            args.push(user.shell.clone());
        }

        if !user.fullname.is_empty() {
            args.push(String::from("-c"));
            args.push(user.fullname.clone());
        }

        let mut steps: Vec<Step> = vec![Step {
            atom: Box::new(Exec {
                command: String::from(cli.to_str().unwrap()),
                arguments: vec![].into_iter().chain(args.clone()).collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if !user.group.is_empty() {
            for group in self.add_to_group(user) {
                steps.push(group);
            }
        }

        steps
    }

    fn add_to_group(&self, user: &UserVariant) -> Vec<Step> {
        let cli = match which("usermod") {
            Ok(c) => c,
            Err(_) => {
                warn!(message = "Could not get the proper user add tool");
                return vec![];
            }
        };

        if user.group.is_empty() {
            warn!(message = "No groups listed to add user to");
            return vec![];
        }

        if user.username.is_empty() {
            warn!(message = "No user specified to add to group(s)");
            return vec![];
        }

        let mut steps: Vec<Step> = vec![];

        for group in user.group.iter() {
            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from(cli.to_str().unwrap()),
                    arguments: vec![String::from("-a"), String::from("-G"), String::from(group)],
                    privileged: true,
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        steps
    }
}

#[cfg(tests)]
mod test {
    use crate::actions::user::*;

    #[test]
    fn test_add_user() {
        let user_provider = LinuxUserProvider {};
        let steps = user_provider.add_user(&UserVariant { username: test });

        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_add_user_no_username() {
        let user_provider = LinuxUserProvider {};
        let steps = user_provider.add_user(&UserVariant {
            // empty for test purposes
        });

        assert_eq!(steps.len(), 0);
    }

    #[test]
    fn test_add_to_group() {
        let user_provider = LinuxUserProvider {};
        let steps = user_provider.add_to_group(&UserVariant {
            username: test,
            group: vec![String::from("testgroup"), String::from("wheel")],
        });

        assert_eq!(steps.len(), 2);
    }
}
