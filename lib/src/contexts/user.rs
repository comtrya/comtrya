use crate::contexts::{Context, ContextProvider};
use anyhow::Result;
use dirs_next::{config_dir, home_dir};

pub struct UserContextProvider {}

impl ContextProvider for UserContextProvider {
    fn get_prefix(&self) -> String {
        String::from("user")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        Ok(vec![
            Context::KeyValueContext(String::from("id"), self.get_uid().to_string()),
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
        ])
    }
}

impl UserContextProvider {
    #[cfg(unix)]
    fn get_uid(&self) -> u32 {
        users::get_current_uid()
    }

    #[cfg(not(unix))]
    fn get_uid(&self) -> u32 {
        0
    }
}
