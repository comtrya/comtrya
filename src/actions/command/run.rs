use crate::actions::{Action, ActionError, ActionResult};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandRun {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,
}

fn get_false() -> bool {
    false
}

impl Action for CommandRun {
    fn run(
        &self,
        _manifest: &crate::manifests::Manifest,
        _context: &tera::Context,
    ) -> Result<ActionResult, ActionError> {
        Ok(crate::utils::command::run_command(
            crate::utils::command::Command {
                name: self.command.clone(),
                args: self.args.clone(),
                require_root: self.sudo,
            },
        ))?
    }
}
