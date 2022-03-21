use crate::{
    contexts::Contexts,
    plugins::{execute_plugin, Plugin},
};
use anyhow::anyhow;
use tracing::error;

use super::Atom;

pub struct ExecPlugin {
    pub plugin: Plugin,
    pub contexts: Contexts,
}

impl std::fmt::Display for ExecPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Execute plugin with name '{}'.", self.plugin.name,)
    }
}

impl Atom for ExecPlugin {
    fn plan(&self) -> bool {
        match execute_plugin(
            &self.plugin,
            &self.contexts,
            crate::plugins::PluginPhase::Plan,
        ) {
            Ok(result) => result,
            Err(err) => {
                error!("{}", err);

                false
            }
        }
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        match execute_plugin(
            &self.plugin,
            &self.contexts,
            crate::plugins::PluginPhase::Run,
        ) {
            Ok(result) => {
                if result {
                    Ok(())
                } else {
                    Err(anyhow!("Execution did not terminate with expected code."))
                }
            }
            Err(err) => Err(err),
        }
    }
}
