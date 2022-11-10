use self::file::FileSideEffect;

pub mod command;
pub mod directory;
pub mod file;
pub mod git;
pub mod http;

pub enum SideEffect {
    None,
    File(FileSideEffect),
}

pub struct Outcome {
    pub should_run: bool,
    pub side_effects: Vec<SideEffect>,
}

pub trait Atom: std::fmt::Display {
    // Determine if this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome>;

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
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            should_run: true,
            side_effects: Vec::new(),
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
