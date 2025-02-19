use super::UserProvider;
use crate::actions::user::{add_group::UserAddGroup, UserVariant};
use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::utilities;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreeBSDUserProvider {}

impl UserProvider for FreeBSDUserProvider {
    fn add_user(&self, user: &UserVariant, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let mut args: Vec<String> = vec![];

        // is a username isn't provided, cant create a new user
        if user.username.is_empty() {
            return Ok(vec![]);
        }

        args.push(String::from("-n"));
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

        // default to setting a randomly generated password
        args.push(String::from("-w"));
        args.push(String::from("random"));

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        let mut steps: Vec<Step> = vec![Step {
            atom: Box::new(Exec {
                command: String::from("/usr/sbin/pw"),
                arguments: vec![String::from("useradd")]
                    .into_iter()
                    .chain(args.clone())
                    .collect(),
                privileged: true,
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }];

        if !user.group.is_empty() {
            let user_groups = UserAddGroup {
                username: user.username.clone(),
                group: user.group.clone(),
                provider: user.provider.clone(),
            };
            for group in self.add_to_group(&user_groups, contexts)? {
                steps.push(group);
            }
        }

        Ok(steps)
    }

    fn add_to_group(&self, user: &UserAddGroup, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let mut steps: Vec<Step> = vec![];

        if user.group.is_empty() {
            warn!(message = "No Groups listed to add user to");
            return Ok(steps);
        }

        if user.username.is_empty() {
            warn!(message = "No user specified to add to group(s)");
            return Ok(steps);
        }

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        for group in user.group.iter() {
            steps.push(Step {
                atom: Box::new(Exec {
                    command: String::from("/usr/sbin/pw"),
                    // arguments: vec![String::from("-a"), String::from("-G"), String::from(group)],
                    arguments: vec![
                        String::from("usermod"),
                        user.username.clone(),
                        String::from("-G"),
                        String::from(group),
                    ],
                    privileged: true,
                    privilege_provider: privilege_provider.clone(),
                    ..Default::default()
                }),
                initializers: vec![],
                finalizers: vec![],
            });
        }

        Ok(steps)
    }
}

#[cfg(target_os = "freebsd")]
#[cfg(test)]
mod test {
    use crate::actions::user::providers::{FreeBSDUserProvider, UserProvider};
    use crate::actions::user::{add_group::UserAddGroup, UserVariant};
    use crate::contexts::Contexts;

    #[test]
    fn test_add_user() {
        let user_provider = FreeBSDUserProvider {};
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

        assert_eq!(steps.unwrap().len(), 1);
    }

    #[test]
    fn test_add_user_no_username() {
        let user_provider = FreeBSDUserProvider {};
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

        assert_eq!(steps.unwrap().len(), 0);
    }

    #[test]
    fn test_add_to_group() {
        let user_provider = FreeBSDUserProvider {};
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
        let user_provider = FreeBSDUserProvider {};
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
