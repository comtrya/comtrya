use std::collections::HashMap;

use crate::actions::{Action, ActionError, ActionResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandRun {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,

    pub dir: String,
}

fn get_false() -> bool {
    false
}

impl Action for CommandRun {
    fn dry_run(
        &self,
        _manifest: &crate::manifests::Manifest,
        _context: &tera::Context,
    ) -> Result<ActionResult, ActionError> {
        let pretty_args = self
            .args
            .iter()
            .map(|a| format!("\"{}\"", a))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(ActionResult {
            message: format!(
                "run {} with args {} (require_root={})",
                self.command, pretty_args, self.sudo
            ),
        })
    }
    fn run(
        &self,
        _manifest: &crate::manifests::Manifest,
        _context: &tera::Context,
        _dry_run: bool,
    ) -> Result<ActionResult, ActionError> {
        crate::utils::command::run_command(crate::utils::command::Command {
            name: self.command.clone(),
            env: HashMap::new(),
            args: self.args.clone(),
            dir: Some(self.dir.clone()),
            require_root: self.sudo,
        })
    }
}
