pub mod command_found;

pub enum FlowControl {
    SkipIf(Box<dyn Initializer>),
}

pub trait Initializer {
    fn run(&self) -> bool;
}
