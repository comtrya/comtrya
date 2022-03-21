pub mod command;
pub mod directory;
pub mod file;
pub mod git;
pub mod http;
pub mod plugin;

pub trait Atom: std::fmt::Display {
    // Determine if this atom needs to run
    fn plan(&self) -> bool;

    // Apply new to old
    fn execute(&mut self) -> anyhow::Result<()>;

    // These methods allow for finalizers to query the outcome of the Atom.
    // We'll provide default implementations to allow Atoms to opt in to
    // the queries that make sense for them
    fn output_string(&self) -> String {
        String::from("")
    }

    fn error_message(&self) -> String {
        String::from("")
    }

    fn status_code(&self) -> i32 {
        0
    }
}

pub struct Echo(pub &'static str);

impl Atom for Echo {
    fn plan(&self) -> bool {
        true
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn output_string(&self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for Echo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Echo: {}", self.0)
    }
}
