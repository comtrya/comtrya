pub mod always_succeed;
pub mod output_contains;

#[allow(dead_code)]
pub enum FlowControl {
    ErrorIf(Box<dyn Finalizer>),
    FinishIf(Box<dyn Finalizer>),
}

pub trait Finalizer {
    fn run(&self, result: &Result<std::process::Output, std::io::Error>) -> bool;
}
