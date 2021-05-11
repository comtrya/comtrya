pub mod always_succeed;

pub enum FlowControl {
    ErrorIf(Box<dyn Finalizer>),
    FinishIf(Box<dyn Finalizer>),
}

pub trait Finalizer {
    fn run(&self, result: &Result<std::process::Output, std::io::Error>) -> bool;
}
