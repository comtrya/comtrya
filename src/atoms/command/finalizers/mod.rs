pub mod always_suceed;

pub enum FlowControl {
    ErrorIf(Box<dyn Finalizer>),
    FinishIf(Box<dyn Finalizer>),
}

pub trait Finalizer {
    fn run(&self) -> bool;
}
