mod command_found;

#[allow(dead_code)]
pub enum FlowControl {
    SkipIf(Box<dyn Initializer>),
}

/// Initializers allow us to modify or skip the execution of an atom
pub trait Initializer {
    fn initialize(&self) -> anyhow::Result<bool>;
}
