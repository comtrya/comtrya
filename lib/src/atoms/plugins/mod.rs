use std::fmt::Display;

use comtrya_plugins::Function;

use crate::plugins::FunctionProxy;

use super::Atom;

pub struct PluginAtom {
    pub name: String,
    pub arguments: Vec<String>,
    pub function: FunctionProxy,
}

impl Display for PluginAtom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plugin: {}", self.name)
    }
}

impl Atom for PluginAtom {
    fn plan(&self) -> bool {
        matches!(self.function.plan(&self.arguments), Ok(true))
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        self.function
            .run(&self.arguments)
            .map_err(|_| anyhow::anyhow!("Failed to run plugin {}", self.name)) // FIXME: Show error
    }
}
