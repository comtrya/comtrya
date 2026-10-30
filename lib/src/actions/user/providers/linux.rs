//! `useradd` and `usermod` on Linux

use super::UserProvider;
use crate::actions::{find, Error, Result};
use crate::contexts::Facts;
use crate::steps::Step;
use crate::{
    actions::user::add_group::UserAddGroup, actions::user::UserVariant, atoms::command::Exec,
};
use serde::{Deserialize, Serialize};

/// Creates users with `useradd` and changes them with `usermod`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinuxUserProvider {}

impl UserProvider for LinuxUserProvider {
    fn add_user(&self, user: &UserVariant, facts: &Facts) -> Result<Vec<Step>> {
        if user.username.is_empty() {
            return Err(Error::Missing {
                action: "user.add",
                field: "username",
            });
        }
        let cli = find("useradd")?;

        let mut args: Vec<String> = vec![];

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

        let mut steps: Vec<Step> = vec![Step::new(Exec {
            command: cli.display().to_string(),
            arguments: vec![].into_iter().chain(args.clone()).collect(),
            ..Default::default()
        })
        .as_root()];

        if !user.group.is_empty() {
            let user_groups = UserAddGroup {
                username: user.username.clone(),
                group: user.group.clone(),
                provider: user.provider.clone(),
            };
            for group in self.add_to_group(&user_groups, facts)? {
                steps.push(group);
            }
        }

        Ok(steps)
    }

    fn add_to_group(&self, user: &UserAddGroup, _facts: &Facts) -> Result<Vec<Step>> {
        super::check_add_to_group(user)?;
        let cli = find("usermod")?;

        let mut steps: Vec<Step> = vec![];

        for group in user.group.iter() {
            steps.push(
                Step::new(Exec {
                    command: cli.display().to_string(),
                    arguments: vec![
                        String::from("-a"),
                        String::from("-G"),
                        String::from(group),
                        user.username.clone(),
                    ],
                    ..Default::default()
                })
                .as_root(),
            );
        }

        Ok(steps)
    }
}

#[cfg(target_os = "linux")]
#[cfg(test)]
mod test {
    use crate::actions::user::providers::{LinuxUserProvider, UserProvider};
    use crate::actions::user::{add_group::UserAddGroup, UserVariant};
    use crate::contexts::Facts;

    #[test]
    fn test_add_user() {
        let user_provider = LinuxUserProvider {};
        let facts = Facts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from("test"),
                shell: String::from("sh"),
                home_dir: String::from("/home/test"),
                fullname: String::from("Test User"),
                group: vec![],
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn test_add_user_no_username_fails() {
        let user_provider = LinuxUserProvider {};
        let facts = Facts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from(""),
                shell: String::from("sh"),
                home_dir: String::from("/home/test"),
                fullname: String::from("Test User"),
                group: vec![],
                ..Default::default()
            },
            &facts,
        );

        assert!(steps.is_err());
    }

    #[test]
    fn test_add_to_group() {
        let user_provider = LinuxUserProvider {};
        let facts = Facts::default();
        let steps = user_provider.add_to_group(
            &UserAddGroup {
                username: String::from("test"),
                group: vec![String::from("testgroup"), String::from("wheel")],
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_create_user_add_to_group() {
        let user_provider = LinuxUserProvider {};
        let facts = Facts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from("test"),
                shell: String::from(""),
                home_dir: String::from(""),
                fullname: String::from(""),
                group: vec![String::from("testgroup")],
                ..Default::default()
            },
            &facts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }
}
