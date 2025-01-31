pub mod command;
pub mod directory;
pub mod file;
pub mod git;
pub mod http;

use crate::utilities::password_manager::PasswordManager;

pub enum SideEffect {}

pub struct Outcome {
    pub side_effects: Vec<SideEffect>,
    pub should_run: bool,
}

#[async_trait::async_trait]
pub trait Atom: std::fmt::Display + Send + Sync {
    fn plan(&self) -> anyhow::Result<Outcome>;

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()>;

    fn output_string(&self) -> String {
        String::from("")
    }

    fn error_message(&self) -> String {
        String::from("")
    }

    fn status_code(&self) -> i32 {
        0
    }
}

pub struct Echo(pub &'static str);

#[async_trait::async_trait]
impl Atom for Echo {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        Ok(())
    }

    fn output_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for Echo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Echo: {}", self.0)
    }
}
