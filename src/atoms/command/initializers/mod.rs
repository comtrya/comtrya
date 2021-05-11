pub mod command_found;

#[allow(dead_code)]
pub enum FlowControl {
    SkipIf(Box<dyn Initializer>),
}

pub trait Initializer {
    fn run(&self) -> bool;
}
