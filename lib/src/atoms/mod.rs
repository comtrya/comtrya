//! Atoms: the smallest changes comtrya makes, like writing a file or running a command. Actions
//! plan them, wrapped in steps

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

/// Running programs
pub mod command;
/// Creating and removing directories
pub mod directory;
/// Writing, linking, and changing files
pub mod file;
/// Cloning repositories
pub mod git;
/// Downloading files
pub mod http;

/// What an atom changes besides its target. Nothing reports any yet
pub enum SideEffect {}

/// What planning an atom found
pub struct Outcome {
    /// Always empty for now
    pub side_effects: Vec<SideEffect>,
    /// Whether the system differs from what the atom wants, so it needs to run
    pub should_run: bool,
}

/// One change to the system. `Display` describes it in the output
pub trait Atom: std::fmt::Display {
    /// Whether this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome>;

    /// Makes the change
    fn execute(&mut self) -> anyhow::Result<()>;

    /// What `execute` would change, e.g. a diff of a file's contents. Only asked for
    /// atoms that need to run
    fn describe_change(&self) -> Option<String> {
        None
    }

    // Finalizers query the outcome of an atom with the methods below. Atoms implement the
    // ones that make sense for them

    /// The output of the atom, like a command's stdout
    fn output_string(&self) -> String {
        String::from("")
    }

    /// The error output of the atom, like a command's stderr
    fn error_message(&self) -> String {
        String::from("")
    }

    /// The exit code of the atom. Nothing sets it yet
    fn status_code(&self) -> i32 {
        0
    }
}

/// Does nothing, and has its text as output. For tests
pub struct Echo(pub &'static str);

impl Atom for Echo {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
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
