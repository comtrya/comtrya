//! What actions registered (manifest version 2): `register: rustc` on a `command.run` keeps
//! what the command printed and how it exited, and later templates and `where` conditions
//! read it as `registered.rustc.stdout`, `registered.rustc.stderr` and
//! `registered.rustc.exit_code`.
//!
//! The values exist once the action ran, so they are per run, for every manifest that runs
//! after it: the later actions of its manifest, and the manifests that depend on it. With
//! `--jobs`, a manifest that doesn't depend on it may run before or at the same time, so only
//! dependents can rely on the values.
//!
//! ```
//! use comtrya_lib::registered::{missing, Output, Registers};
//!
//! let registers = Registers::from([(String::from("rustc"), Output::default())]);
//!
//! assert_eq!(None, missing("{{ registered.rustc.stdout }}", &registers));
//! assert_eq!(Some("cargo"), missing("registered.cargo.exit_code == 0", &registers));
//! ```

use serde::Serialize;
use std::collections::BTreeMap;

/// What one command printed and how it exited, see the module docs
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Output {
    /// What it printed to stdout: its lines, joined with line breaks, without the last one
    pub stdout: String,
    /// What it printed to stderr, the same way
    pub stderr: String,
    /// Its exit code: 0, or the code it failed with when its action has `continue_on_error`
    pub exit_code: i32,
}

/// What the actions of a run registered so far, by the name of their `register`
pub type Registers = BTreeMap<String, Output>;

/// No action registered anything, the start of every run
pub static NONE: Registers = BTreeMap::new();

/// The names that `text`, a template or a `where` condition, reads with `registered.<name>`
pub fn names_in(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("registered.")
        .filter_map(|(start, prefix)| {
            // `registered` is a whole word, not the end of e.g. `unregistered`
            let before = text[..start].chars().next_back();
            if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                return None;
            }
            let rest = &text[start + prefix.len()..];
            let end = rest.find(|c: char| !is_name_char(c)).unwrap_or(rest.len());
            (end > 0).then(|| &rest[..end])
        })
}

/// The first name `text` reads that no action registered yet, see [`names_in`]
pub fn missing<'a>(text: &'a str, registers: &Registers) -> Option<&'a str> {
    names_in(text).find(|name| !registers.contains_key(*name))
}

/// Whether `name` can be a `register` name: letters, digits and `_`, not starting with a digit,
/// so templates and conditions can read it as `registered.<name>`
pub fn is_name(name: &str) -> bool {
    name.chars().next().is_some_and(|c| !c.is_ascii_digit()) && name.chars().all(is_name_char)
}

/// Whether `c` can be in a `register` name
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn names_are_found_in_templates_and_conditions() {
        let names: Vec<_> =
            names_in("{{ registered.a.stdout }} and registered.b_2.exit_code, not unregistered.c")
                .collect();

        assert_eq!(vec!["a", "b_2"], names);
    }

    #[test]
    fn the_first_unregistered_name_is_missing() {
        let registers = Registers::from([(String::from("a"), Output::default())]);

        assert_eq!(
            Some("b"),
            missing("registered.a.stdout registered.b", &registers)
        );
        assert_eq!(None, missing("no values", &registers));
    }

    #[test]
    fn names_are_words() {
        assert!(is_name("rustc_version"));
        assert!(!is_name("1st"));
        assert!(!is_name("rust-c"));
        assert!(!is_name(""));
    }
}
