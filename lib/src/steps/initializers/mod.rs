mod command_found;
pub use command_found::CommandFound;

mod file_exists;
pub use file_exists::FileExists;

#[allow(dead_code)]
pub enum FlowControl {
    Ensure(Box<dyn Initializer>),
    SkipIf(Box<dyn Initializer>),
}

/// Initializers allow us to modify or skip the execution of an atom
pub trait Initializer {
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
