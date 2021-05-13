pub mod command;
pub mod file;

pub trait Atom: std::fmt::Display {
    // Determine if this atom needs to run
    fn plan(&self) -> bool;

    // Apply new to old
    fn execute(&self) -> anyhow::Result<()>;
}
