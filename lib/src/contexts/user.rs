use crate::contexts::{Context, ContextProvider};
use anyhow::Result;
use dirs_next::{config_dir, data_dir, data_local_dir, document_dir, home_dir};

pub struct UserContextProvider {}

impl ContextProvider for UserContextProvider {
    fn get_prefix(&self) -> String {
        String::from("user")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        Ok(vec![
            Context::KeyValueContext(String::from("id"), self.get_uid().to_string().into()),
            Context::KeyValueContext(String::from("name"), whoami::realname().into()),
            Context::KeyValueContext(String::from("username"), whoami::username().into()),
            Context::KeyValueContext(
                String::from("home_dir"),
                home_dir()
                    .map(Into::into)
                    .unwrap_or_else(|| "unknown".into()),
            ),
            Context::KeyValueContext(
                String::from("config_dir"),
                config_dir()
                    .map(Into::into)
                    .unwrap_or_else(|| "unknown".into()),
            ),
            Context::KeyValueContext(
                String::from("data_dir"),
                data_dir()
                    .map(Into::into)
                    .unwrap_or_else(|| "unknown".into()),
            ),
            Context::KeyValueContext(
                String::from("data_local_dir"),
                data_local_dir()
                    .map(Into::into)
                    .unwrap_or_else(|| "unknown".into()),
            ),
            Context::KeyValueContext(
                String::from("document_dir"),
                document_dir()
                    .map(Into::into)
                    .unwrap_or_else(|| "unknown".into()),
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
