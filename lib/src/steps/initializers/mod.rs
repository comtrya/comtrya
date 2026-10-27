//! Initializers: checks before an atom runs, which can skip it

/// Whether a program is installed
mod command_found;
pub use command_found::CommandFound;

/// Whether a file exists
mod file_exists;

pub use file_exists::FileExists;

/// How an initializer's answer is used
#[allow(dead_code)]
pub enum FlowControl {
    /// The atom runs only if the initializer says yes
    Ensure(Box<dyn Initializer>),
    /// The atom is skipped if the initializer says yes
    SkipIf(Box<dyn Initializer>),
}

/// A check before an atom runs, which can skip it
pub trait Initializer {
    /// The answer
    fn initialize(&self) -> anyhow::Result<bool>;
}

#[cfg(test)]
pub(crate) mod test {
    use super::Initializer;
    use anyhow::anyhow;

    #[derive(Clone, Debug)]
    pub struct Echo(pub bool);

    impl Initializer for Echo {
        fn initialize(&self) -> anyhow::Result<bool> {
            Ok(self.0)
        }
    }

    #[derive(Clone, Debug)]
    pub struct Error();

    impl Initializer for Error {
        fn initialize(&self) -> anyhow::Result<bool> {
            Err(anyhow!("ErrorInitializer"))
        }
    }
}
