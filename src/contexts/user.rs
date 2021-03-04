use crate::contexts::{Context, ContextProvider};
use dirs::{config_dir, home_dir};

pub struct UserContextProvider {}

impl ContextProvider for UserContextProvider {
    fn get_prefix(&self) -> String {
        String::from("user")
    }

    fn get_contexts(&self) -> Vec<super::Context> {
        vec![
            Context::KeyValueContext(String::from("name"), whoami::realname()),
            Context::KeyValueContext(String::from("username"), whoami::username()),
            Context::KeyValueContext(
                String::from("home_dir"),
                home_dir().unwrap().into_os_string().into_string().unwrap(),
            ),
            Context::KeyValueContext(
                String::from("config_dir"),
                config_dir()
                    .unwrap()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        ]
    }
}
