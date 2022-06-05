use super::User;
use super::UserVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;
use tracing::{error, span};

pub type UserAdd = User;

impl Action for UserAdd {
    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> Vec<Step> {
        let variant: UserVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let span = span!(
            tracing::Level::INFO,
            "user.add",
            // provider = provider.name()
        )
        .entered();

        let mut atoms: Vec<Step> = vec![];

        // If the provider isn't available, see if we can bootstrap it
        // if !provider.available() {
        //     if provider.bootstrap().is_empty() {
        //         error!(
        //             "User Provider, {}, isn't available. Skipping action",
        //             provider.name()
        //         );
        //         return vec![];
        //     }

        //     atoms.append(&mut provider.bootstrap());
        // }

        // if let Some(ref _repo) = variant.repository {
        //     if !provider.has_repository(&variant) {
        //         atoms.append(&mut provider.add_repository(&variant));
        //     }
        // }

        atoms.append(&mut provider.add_user(&variant));

        span.exit();

        atoms
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::actions::Actions;

//     #[test]
//     fn it_can_be_deserialized() {
//         let yaml = r#"
// - action: package.install
//   name: curl

// - action: package.install
//   list:
//     - bash
// "#;

//         let mut actions: Vec<Actions> = serde_yaml::from_str(yaml).unwrap();

//         match actions.pop() {
//             Some(Actions::UserAdd(action)) => {
//                 assert_eq!(vec!["bash"], action.action.list);
//             }
//             _ => {
//                 panic!("UserAdd didn't deserialize to the correct type");
//             }
//         };

//         match actions.pop() {
//             Some(Actions::UserAdd(action)) => {
//                 assert_eq!("curl", action.action.name.unwrap());
//             }
//             _ => {
//                 panic!("UserAdd didn't deserialize to the correct type");
//             }
//         };
//     }
// }
