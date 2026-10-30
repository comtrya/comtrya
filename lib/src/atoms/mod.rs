//! Atoms: the smallest changes comtrya makes, like writing a file or running a command. Actions
//! plan them, wrapped in steps

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

use serde::{Deserialize, Serialize};

/// What an atom changes besides its target. Nothing reports any yet
pub enum SideEffect {}

/// What planning an atom found
pub struct Outcome {
    /// Always empty for now
    pub side_effects: Vec<SideEffect>,
    /// Whether the system differs from what the atom wants, so it needs to run
    pub should_run: bool,
}

/// What running an atom returned, for the finalizers. Empty for atoms that don't run programs
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Output {
    /// What the program wrote to stdout, unless it went to a file
    pub stdout: String,
    /// What the program wrote to stderr
    pub stderr: String,
}

/// One change to the system, as data: actions plan atoms, the executor runs them. It can be
/// written out and read back, so that another process, like a privileged helper, can run it.
/// `Display` describes it in the output
///
/// Careful: [`file::Decrypt`] and [`file::SetContents`] hold secrets as they are
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "atom", rename_all = "snake_case")]
pub enum Atom {
    /// Runs a program
    Exec(command::Exec),
    /// Copies the contents of a directory
    DirectoryCopy(directory::Copy),
    /// Creates a directory
    DirectoryCreate(directory::Create),
    /// Removes a directory
    DirectoryRemove(directory::Remove),
    /// Sets the permissions of a file
    FileChmod(file::Chmod),
    /// Sets the owner and group of a file
    FileChown(file::Chown),
    /// Copies a file
    FileCopy(file::Copy),
    /// Creates an empty file
    FileCreate(file::Create),
    /// Writes age-encrypted contents decrypted
    FileDecrypt(file::Decrypt),
    /// Creates a symlink
    FileLink(file::Link),
    /// Removes a file
    FileRemove(file::Remove),
    /// Writes the contents of a file
    FileSetContents(file::SetContents),
    /// Unpacks an archive
    FileUnarchive(file::Unarchive),
    /// Clones a repository
    GitClone(git::Clone),
    /// Replaces a clone with a fresh one
    GitUpdate(git::Update),
    /// Downloads a file
    HttpDownload(http::Download),
    /// For tests, never written out
    #[cfg(test)]
    #[serde(skip)]
    Echo(Echo),
}

impl Atom {
    /// The atom inside, to call what every atom has
    fn change(&self) -> &dyn Change {
        match self {
            Atom::Exec(atom) => atom,
            Atom::DirectoryCopy(atom) => atom,
            Atom::DirectoryCreate(atom) => atom,
            Atom::DirectoryRemove(atom) => atom,
            Atom::FileChmod(atom) => atom,
            Atom::FileChown(atom) => atom,
            Atom::FileCopy(atom) => atom,
            Atom::FileCreate(atom) => atom,
            Atom::FileDecrypt(atom) => atom,
            Atom::FileLink(atom) => atom,
            Atom::FileRemove(atom) => atom,
            Atom::FileSetContents(atom) => atom,
            Atom::FileUnarchive(atom) => atom,
            Atom::GitClone(atom) => atom,
            Atom::GitUpdate(atom) => atom,
            Atom::HttpDownload(atom) => atom,
            #[cfg(test)]
            Atom::Echo(atom) => atom,
        }
    }

    /// Whether this atom needs to run
    pub fn plan(&self) -> anyhow::Result<Outcome> {
        self.change().plan()
    }

    /// Makes the change
    pub fn execute(&self) -> anyhow::Result<Output> {
        self.change().execute_with_output()
    }

    /// What `execute` would change, e.g. a diff of a file's contents. Only asked for atoms that
    /// need to run
    pub fn describe_change(&self) -> Option<String> {
        self.change().describe_change()
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.change().fmt(f)
    }
}

/// `impl From<Struct> for Atom` for every atom, so actions plan with `Step::new(file::Copy { … })`
macro_rules! atom_from {
    ($($variant:ident($atom:ty)),* $(,)?) => {
        $(
            impl From<$atom> for Atom {
                fn from(atom: $atom) -> Self {
                    Atom::$variant(atom)
                }
            }
        )*
    };
}

atom_from!(
    Exec(command::Exec),
    DirectoryCopy(directory::Copy),
    DirectoryCreate(directory::Create),
    DirectoryRemove(directory::Remove),
    FileChmod(file::Chmod),
    FileChown(file::Chown),
    FileCopy(file::Copy),
    FileCreate(file::Create),
    FileDecrypt(file::Decrypt),
    FileLink(file::Link),
    FileRemove(file::Remove),
    FileSetContents(file::SetContents),
    FileUnarchive(file::Unarchive),
    GitClone(git::Clone),
    GitUpdate(git::Update),
    HttpDownload(http::Download),
);

#[cfg(test)]
atom_from!(Echo(Echo));

/// What every atom does. [`Atom`] calls it on the atom inside
trait Change: std::fmt::Display {
    /// Whether this atom needs to run
    fn plan(&self) -> anyhow::Result<Outcome>;

    /// Makes the change
    fn execute(&self) -> anyhow::Result<()>;

    /// Makes the change and returns what the finalizers look at. Atoms that run programs
    /// return their output
    fn execute_with_output(&self) -> anyhow::Result<Output> {
        self.execute().map(|()| Output::default())
    }

    /// What `execute` would change, e.g. a diff of a file's contents
    fn describe_change(&self) -> Option<String> {
        None
    }
}

/// Does nothing, and has its text as output. For tests
#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub struct Echo(pub &'static str);

#[cfg(test)]
impl Change for Echo {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn execute_with_output(&self) -> anyhow::Result<Output> {
        Ok(Output {
            stdout: self.0.to_string(),
            stderr: String::new(),
        })
    }
}

#[cfg(test)]
impl std::fmt::Display for Echo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Echo: {}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::PathBuf;

    /// One atom of every kind
    fn every_atom() -> Vec<Atom> {
        let path = || PathBuf::from("/tmp/comtrya");
        let repository =
            || gix::Url::from_bytes("https://github.com/comtrya/comtrya".into()).unwrap();

        vec![
            command::Exec {
                command: String::from("echo"),
                arguments: vec![String::from("hi")],
                working_dir: Some(String::from("/tmp")),
                environment: vec![(String::from("A"), String::from("b"))],
                privileged: true,
                privilege_provider: String::from("doas"),
                stdout_file: Some(path()),
            }
            .into(),
            directory::Copy {
                from: path(),
                to: path(),
            }
            .into(),
            directory::Create { path: path() }.into(),
            directory::Remove { target: path() }.into(),
            file::Chmod {
                path: path(),
                mode: 0o644,
            }
            .into(),
            file::Chown {
                path: path(),
                owner: String::from("me"),
                group: String::from("staff"),
            }
            .into(),
            file::Copy {
                from: path(),
                to: path(),
            }
            .into(),
            file::Create { path: path() }.into(),
            file::Decrypt {
                encrypted_content: vec![1, 2, 3],
                passphrase: String::from("pass"),
                path: path(),
            }
            .into(),
            file::Link {
                source: path(),
                target: path(),
            }
            .into(),
            file::Remove { target: path() }.into(),
            file::SetContents {
                path: path(),
                contents: b"text".to_vec(),
                redact: true,
            }
            .into(),
            file::Unarchive {
                origin: path(),
                dest: path(),
                force: true,
            }
            .into(),
            git::Clone {
                repository: repository(),
                directory: path(),
                reference: Some(String::from("main")),
            }
            .into(),
            git::Update {
                repository: repository(),
                directory: path(),
                reference: None,
            }
            .into(),
            http::Download {
                url: String::from("https://comtrya.dev/install.sh"),
                to: path(),
                sha256: None,
            }
            .into(),
        ]
    }

    #[test]
    fn every_atom_survives_being_written_and_read_back() {
        for atom in every_atom() {
            let json = serde_json::to_string(&atom).unwrap();

            assert_eq!(atom, serde_json::from_str::<Atom>(&json).unwrap(), "{json}");
        }
    }

    #[test]
    fn the_kind_of_atom_is_a_field() {
        let atom = Atom::from(file::Create {
            path: PathBuf::from("/tmp/x"),
        });

        assert_eq!(
            r#"{"atom":"file_create","path":"/tmp/x"}"#,
            serde_json::to_string(&atom).unwrap()
        );
    }

    #[test]
    fn a_git_url_is_written_as_a_string() {
        let atom = every_atom()
            .into_iter()
            .find(|atom| matches!(atom, Atom::GitClone(_)))
            .unwrap();

        let json = serde_json::to_value(&atom).unwrap();
        assert_eq!("https://github.com/comtrya/comtrya", json["repository"]);
    }
}
