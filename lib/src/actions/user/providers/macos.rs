//! `dscl` on macOS

use super::UserProvider;
use crate::actions::user::{add_group::UserAddGroup, UserVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use which::which;

/// Creates and changes users with `dscl`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSUserProvider {}

impl UserProvider for MacOSUserProvider {
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        if user.username.is_empty() {
            bail!("can't create a user without a username");
        }
        let cli = which("dscl").context("can't find `dscl` to create users with")?;

        let mut args: Vec<String> = vec![];

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
            args.push(user.fullname.clone());
        }

        let privilege_provider = utilities::get_privilege_provider(contexts);

        let mut steps: Vec<Step> = vec![Step::new(Exec {
            command: cli.display().to_string(),
            arguments: vec![].into_iter().chain(args.clone()).collect(),
            privileged: true,
            privilege_provider: privilege_provider.clone(),
            ..Default::default()
        })];

        if !user.group.is_empty() {
            let user_group = UserAddGroup {
                username: user.username.clone(),
                group: user.group.clone(),
                provider: user.provider.clone(),
            };
            for group in self.add_to_group(&user_group, contexts)? {
                steps.push(group);
            }
        }

        Ok(steps)
    }

    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        super::check_add_to_group(user)?;
        let cli = which("dscl").context("can't find `dscl` to add users to groups with")?;

        let privilege_provider = utilities::get_privilege_provider(contexts);

        let mut steps: Vec<Step> = vec![];

        for group in user.group.iter() {
            let mut group_string: String = String::from("/Groups/");
            group_string.push_str(&group.clone());

            let args: Vec<String> = vec![
                ".".to_string(),
                "append".to_string(),
                group_string.clone(),
                "GroupMembership".to_string(),
                user.username.clone(),
            ];

            steps.push(Step::new(Exec {
                command: cli.display().to_string().clone(),
                arguments: args.into_iter().collect(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }));
        }

        Ok(steps)
    }
}

#[cfg(target_os = "macos")]
#[cfg(test)]
mod test {
    use crate::actions::user::providers::{MacOSUserProvider, UserProvider};
    use crate::actions::user::{add_group::UserAddGroup, UserVariant};
    use crate::contexts::Contexts;

    #[test]
    fn test_add_user() {
        let user_provider = MacOSUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from("test"),
                shell: String::from("sh"),
                home_dir: String::from("/home/test"),
                fullname: String::from("Test User"),
                group: vec![],
                ..Default::default()
            },
            &contexts,
        );

        let steps = steps.unwrap();
        assert_eq!(steps.len(), 1);
        assert!(
            steps[0].atom.to_string().contains("RealName Test User"),
            "{}",
            steps[0].atom
        );
    }

    #[test]
    fn test_add_user_no_username_fails() {
        let user_provider = MacOSUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from(""),
                shell: String::from("sh"),
                home_dir: String::from("/home/test"),
                fullname: String::from("Test User"),
                group: vec![],
                ..Default::default()
            },
            &contexts,
        );

        assert!(steps.is_err());
    }

    #[test]
    fn test_add_to_group() {
        let user_provider = MacOSUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider.add_to_group(
            &UserAddGroup {
                username: String::from("test"),
                group: vec![String::from("testgroup"), String::from("wheel")],
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }

    #[test]
    fn test_create_user_add_to_group() {
        let user_provider = MacOSUserProvider {};
        let contexts = Contexts::default();
        let steps = user_provider.add_user(
            &UserVariant {
                username: String::from("test"),
                shell: String::from(""),
                home_dir: String::from(""),
                fullname: String::from(""),
                group: vec![String::from("testgroup")],
                ..Default::default()
            },
            &contexts,
        );

        assert_eq!(steps.unwrap().len(), 2);
    }
}
