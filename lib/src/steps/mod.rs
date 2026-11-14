//! Steps: the atoms an action plans, and who runs them

use crate::atoms::{self, Atom, Resource, RunAs};
use crate::contexts::privilege::Privilege;
use crate::utilities;

/// One change to the system: an atom, and who makes it. Whether it runs is up to the atom's
/// check, see [`Atom::check`]
pub struct Step {
    /// The change to make
    pub atom: Atom,
    /// Who makes it
    pub run_as: RunAs,
}

impl Step {
    /// A step that runs `atom` as the user
    pub fn new(atom: impl Into<Atom>) -> Self {
        Step {
            atom: atom.into(),
            run_as: RunAs::User,
        }
    }

    /// Runs the atom as root
    pub fn as_root(mut self) -> Self {
        self.run_as = RunAs::Root;
        self
    }

    /// The atom to execute: a root step runs through `privilege`, unless comtrya runs as root
    /// already
    pub fn atom_to_run(&self, privilege: &Privilege) -> atoms::Result<Atom> {
        self.atom_to_run_with(privilege, utilities::running_as_root())
    }

    /// Whether the atom has to run in the privileged helper: a root step that isn't a program,
    /// unless comtrya runs as root already. Programs run through the privilege provider, see
    /// [`Step::atom_to_run`]
    pub fn needs_helper(&self) -> bool {
        self.needs_helper_with(utilities::running_as_root())
    }

    /// [`Step::needs_helper`], with whether comtrya runs as root
    fn needs_helper_with(&self, as_root: bool) -> bool {
        self.run_as == RunAs::Root && !as_root && !matches!(self.atom, Atom::Exec(_))
    }

    /// [`Step::atom_to_run`], with whether comtrya runs as root
    fn atom_to_run_with(&self, privilege: &Privilege, as_root: bool) -> atoms::Result<Atom> {
        match self.run_as {
            RunAs::Root if !as_root => self.atom.clone().elevated(privilege),
            _ => Ok(self.atom.clone()),
        }
    }

    /// What the step uses, see [`Resource`]: its atom's resources, and the terminal for a root
    /// step, because `sudo` may ask for a password
    pub fn resources(&self) -> Vec<Resource> {
        let mut resources = self.atom.resources();
        if self.run_as == RunAs::Root {
            resources.push(Resource::Tty);
        }
        resources
    }
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.run_as {
            RunAs::User => write!(f, "{}", self.atom),
            RunAs::Root => write!(f, "{} (as root)", self.atom),
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn a_root_step_needs_the_terminal() {
        let pacman = atoms::command::Exec {
            command: String::from("pacman"),
            ..Default::default()
        };

        assert_eq!(
            vec![Resource::PackageDb(String::from("pacman"))],
            Step::new(pacman.clone()).resources()
        );
        assert_eq!(
            vec![Resource::PackageDb(String::from("pacman")), Resource::Tty],
            Step::new(pacman).as_root().resources()
        );
    }

    #[test]
    fn a_copy_uses_both_of_its_paths() {
        let copy = atoms::file::Copy {
            from: std::path::PathBuf::from("/a"),
            to: std::path::PathBuf::from("/b"),
        };

        assert_eq!(
            vec![
                Resource::Path(std::path::PathBuf::from("/a")),
                Resource::Path(std::path::PathBuf::from("/b"))
            ],
            Step::new(copy).resources()
        );
    }

    /// `echo hi`, as root
    fn root_echo() -> Step {
        let mut exec = crate::atoms::command::Exec {
            command: String::from("echo"),
            ..Default::default()
        };
        exec.arguments = vec![String::from("hi")];
        Step::new(exec).as_root()
    }

    #[test]
    fn a_root_step_runs_through_the_privilege_provider() {
        let atom = root_echo()
            .atom_to_run_with(&Privilege::Doas, false)
            .unwrap();

        assert_eq!("Run doas echo hi", atom.to_string());
    }

    /// Root is found by its uid, so also under another name or without a passwd entry
    #[test]
    fn a_root_step_runs_as_it_is_when_comtrya_is_root() {
        let atom = root_echo()
            .atom_to_run_with(&Privilege::Doas, true)
            .unwrap();

        assert_eq!("Run echo hi", atom.to_string());
    }

    #[test]
    fn a_user_step_runs_as_it_is() {
        let step = Step::new(crate::atoms::command::Exec {
            command: String::from("id"),
            ..Default::default()
        });

        assert_eq!(
            "Run id",
            step.atom_to_run_with(&Privilege::Sudo, false)
                .unwrap()
                .to_string()
        );
    }

    #[test]
    fn a_root_step_says_so() {
        assert_eq!("Run echo hi (as root)", root_echo().to_string());
    }

    #[test]
    fn only_root_steps_that_arent_programs_need_the_helper() {
        let root_file = Step::new(crate::atoms::file::Create {
            path: "/etc/x".into(),
        })
        .as_root();

        assert!(root_file.needs_helper_with(false));
        assert!(!root_file.needs_helper_with(true));
        assert!(!root_echo().needs_helper_with(false));
        assert!(!Step::new(crate::atoms::file::Create {
            path: "/tmp/x".into()
        })
        .needs_helper_with(false));
    }
}
