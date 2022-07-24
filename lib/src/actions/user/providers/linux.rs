use super::UserProvider;
use crate::steps::Step;
use crate::{actions::user::UserVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LinuxUserProvider {}

impl UserProvider for LinuxUserProvider {
    fn add_user(&self, user: &UserVariant) -> Vec<Step> {
        let mut args: Vec<String> = vec![];

        // is a user name isn't provided, cant create a new user
        if user.username.is_empty() {
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

        vec![Step {
            atom: Box::new(Exec {
                command: String::from("/usr/sbin/useradd"),
                arguments: vec![]
                    .into_iter()
                    .chain(args.clone())
                    .collect(),
                privileged: true,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
