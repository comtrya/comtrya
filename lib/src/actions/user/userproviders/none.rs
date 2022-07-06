use super::UserProvider;
use crate::steps::Step;
use crate::{actions::user::UserVariant, atoms::command::Exec};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoneUserProvider {}

impl UserProvider for NoneUserProvider {
    fn add_user(&self, _user: &UserVariant) -> Vec<Step> {
        vec![Step {
            atom: Box::new(Exec {
                command: String::from("/bin/echo"),
                arguments: vec![String::from("Hello World")],
                privileged: false,
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
