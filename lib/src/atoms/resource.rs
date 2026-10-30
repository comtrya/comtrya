//! What an atom touches, so a scheduler can run atoms at the same time that can't get in each
//! other's way

use super::command::Exec;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Something an atom uses that another atom running at the same time must not use too
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Resource {
    /// A file or directory the atom reads or changes, with everything below it
    Path(PathBuf),
    /// The database of a package manager, like `dpkg` or `pacman`. Package managers lock it, a
    /// second one at the same time fails or waits
    PackageDb(String),
    /// The terminal, for a program that may ask something, like `sudo` for a password
    Tty,
    /// Anything: a program comtrya knows nothing about may change any file
    Anything,
}

impl Resource {
    /// Whether two atoms that use `self` and `other` must not run at the same time
    pub fn conflicts(&self, other: &Resource) -> bool {
        match (self, other) {
            (Resource::Anything, _) | (_, Resource::Anything) => true,
            (Resource::Path(a), Resource::Path(b)) => a.starts_with(b) || b.starts_with(a),
            (Resource::PackageDb(a), Resource::PackageDb(b)) => a == b,
            (Resource::Tty, Resource::Tty) => true,
            _ => false,
        }
    }
}

/// [`Resource::Path`] of `path`
pub(super) fn path(path: &Path) -> Resource {
    Resource::Path(path.to_path_buf())
}

/// What a program uses: the package database and maybe the terminal for a package manager,
/// anything for every other program. And the file stdout goes to
pub(super) fn of_exec(exec: &Exec) -> Vec<Resource> {
    let mut resources = match package_db(&exec.command) {
        Some(db) if asks_for_sudo(&exec.command) => vec![Resource::PackageDb(db), Resource::Tty],
        Some(db) => vec![Resource::PackageDb(db)],
        None => vec![Resource::Anything],
    };
    resources.extend(exec.stdout_file.as_deref().map(path));
    resources
}

/// The name of a program without its directory, like `pkg` for `/usr/sbin/pkg`
fn program(command: &str) -> &str {
    Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
}

/// The database that `command` changes, if it's a package manager comtrya plans steps for
fn package_db(command: &str) -> Option<String> {
    let db = match program(command) {
        "apt" | "apt-get" | "dpkg" => "dpkg",
        "pacman" | "yay" | "paru" | "makepkg" => "pacman",
        "dnf" | "yum" | "rpm" | "zypper" => "rpm",
        "brew" => "homebrew",
        "pkg" => "pkg",
        "pkgin" => "pkgin",
        "port" => "macports",
        "snap" => "snap",
        "winget" => "winget",
        "xbps-install" => "xbps",
        _ => return None,
    };
    Some(db.to_string())
}

/// AUR helpers run as the user and call `sudo` themselves, which may ask for a password
fn asks_for_sudo(command: &str) -> bool {
    matches!(program(command), "yay" | "paru" | "makepkg")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn exec(command: &str) -> Exec {
        Exec {
            command: command.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn a_path_conflicts_with_what_is_below_and_above_it() {
        let home = path(Path::new("/home/ferris"));
        let rc = path(Path::new("/home/ferris/.zshrc"));
        let other = path(Path::new("/home/ferrisx"));

        assert!(home.conflicts(&rc));
        assert!(rc.conflicts(&home));
        assert!(!home.conflicts(&other));
    }

    #[test]
    fn package_databases_conflict_only_with_themselves() {
        let dpkg = Resource::PackageDb(String::from("dpkg"));

        assert!(dpkg.conflicts(&dpkg.clone()));
        assert!(!dpkg.conflicts(&Resource::PackageDb(String::from("rpm"))));
        assert!(!dpkg.conflicts(&Resource::Tty));
    }

    #[test]
    fn anything_conflicts_with_everything() {
        assert!(Resource::Anything.conflicts(&Resource::Tty));
        assert!(path(Path::new("/x")).conflicts(&Resource::Anything));
    }

    #[test]
    fn package_managers_use_their_database() {
        assert_eq!(
            vec![Resource::PackageDb(String::from("pkg"))],
            of_exec(&exec("/usr/sbin/pkg"))
        );
        assert_eq!(
            vec![Resource::PackageDb(String::from("pacman")), Resource::Tty],
            of_exec(&exec("paru"))
        );
    }

    #[test]
    fn other_programs_may_use_anything() {
        let mut exec = exec("bash");
        exec.stdout_file = Some(PathBuf::from("/tmp/out"));

        assert_eq!(
            vec![Resource::Anything, path(Path::new("/tmp/out"))],
            of_exec(&exec)
        );
    }
}
