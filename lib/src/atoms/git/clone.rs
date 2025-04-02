use super::super::Atom;
use crate::atoms::Outcome;
use crate::utilities::password_manager::PasswordManager;
use gix::interrupt;
use gix::{progress::Discard, Url};
use std::path::PathBuf;
use tokio::task;
use tracing::instrument;

#[derive(Default)]
pub struct Clone {
    pub repository: Url,
    pub directory: PathBuf,
}

impl std::fmt::Display for Clone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GitClone {} to {}",
            self.repository,
            self.directory.display()
        )
    }
}

#[async_trait::async_trait]
impl Atom for Clone {
    #[instrument(name = "git.clone.plan", level = "info", skip(self))]
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: !self.directory.exists(),
        })
    }

    #[instrument(name = "git.clone.execute", level = "info", skip(self))]
    async fn execute(&mut self, _: Option<PasswordManager>) -> anyhow::Result<()> {
        unsafe {
            interrupt::init_handler(1, || {})?;
        };

        std::fs::create_dir_all(&self.directory)?;

        let repo = self.repository.clone();
        let directory = self.directory.clone();

        // I wonder if this will cause tokio to freak out
        let (mut prepare_checkout, _) = task::spawn_blocking(move || {
            let mut prepare_clone = gix::prepare_clone(repo, &directory).unwrap();
            prepare_clone
                .fetch_then_checkout(gix::progress::Discard, &interrupt::IS_INTERRUPTED)
                .unwrap()
        })
        .await?;

        let (repo, _) = prepare_checkout.main_worktree(Discard, &interrupt::IS_INTERRUPTED)?;

        let _ = repo
            .find_default_remote(gix::remote::Direction::Fetch)
            .expect("always present after clone")?;

        Ok(())
    }

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
