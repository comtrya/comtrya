//! In format 1, `from: ./zshrc` is below the manifest's `files/` directory, like `from: zshrc`.
//! In format 2, `./` and `../` are next to the manifest file. Such paths are rewritten so they
//! point to the same file

use super::quote::has_template;
use serde_yaml_ng::{Mapping, Value};
use std::path::{Component, Path};

/// How an action reads its source path in format 1
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Base {
    /// Below the manifest's `files/` directory
    Files,
    /// As written, relative to where comtrya runs: `file.unarchive`
    AsWritten,
}

/// The source fields: (action, field, how format 1 reads it)
const SOURCES: [(&str, &str, Base); 4] = [
    ("file.copy", "from", Base::Files),
    ("file.link", "from", Base::Files),
    ("directory.copy", "from", Base::Files),
    ("file.unarchive", "from", Base::AsWritten),
];

/// How the action `name` reads its source `field`, `None` for other fields
pub(super) fn base(name: &str, field: &str) -> Option<Base> {
    SOURCES
        .iter()
        .find(|(action, source, _)| *action == name && *source == field)
        .map(|(_, _, base)| *base)
}

/// Rewrites the source paths of the action entry `action` at `at`, called `name`, and of its
/// variants
pub(super) fn rewrite(name: &str, action: &mut Mapping, at: &str) -> Result<(), String> {
    rewrite_fields(name, action, at)?;
    if let Some(Value::Sequence(variants)) = action.get_mut("variants") {
        for (index, variant) in variants.iter_mut().enumerate() {
            if let Value::Mapping(variant) = variant {
                rewrite_fields(name, variant, &format!("{at}.variants[{index}]"))?;
            }
        }
    }
    Ok(())
}

/// Rewrites the source paths among `fields`
fn rewrite_fields(name: &str, fields: &mut Mapping, at: &str) -> Result<(), String> {
    for (key, value) in fields.iter_mut() {
        let (Some(field), Value::String(path)) = (key.as_str(), &mut *value) else {
            continue;
        };
        let Some(base) = base(name, field) else {
            continue;
        };
        if let Some(upgraded) =
            version_2_path(base, path).map_err(|err| format!("{at}.{field}: {err}"))?
        {
            *path = upgraded;
        }
    }
    Ok(())
}

/// The format 2 path that means the format 1 `path`, `None` if it stays as it is
fn version_2_path(base: Base, path: &str) -> Result<Option<String>, String> {
    if !starts_with_dot(path) {
        return Ok(None);
    }
    if has_template(path) {
        return Err(format!(
            "`{path}` starts with `./` or `../` and has a template; write the path without them by hand"
        ));
    }
    match base {
        Base::Files => Ok(Some(next_to_manifest(&normalized(&format!("files/{path}"))))),
        Base::AsWritten => match normalized(path).as_str() {
            "" => Err(format!("`{path}` is the directory comtrya runs in; write it as an absolute path")),
            plain if !plain.starts_with("..") => Ok(Some(plain.to_string())),
            _ => Err(format!(
                "`{path}` is relative to where comtrya runs in version 1, but to the manifest in version 2; write it as an absolute path"
            )),
        },
    }
}

/// Whether `path` starts with `./` or `../`
pub(super) fn starts_with_dot(path: &str) -> bool {
    matches!(
        Path::new(path).components().next(),
        Some(Component::CurDir | Component::ParentDir)
    )
}

/// A path relative to the manifest file's directory, `normalized`, as format 2 writes it: plain
/// below `files/`, with `./` or `../` otherwise
fn next_to_manifest(normalized: &str) -> String {
    match normalized.strip_prefix("files/") {
        Some(below_files) => below_files.to_string(),
        None if normalized.starts_with("..") => normalized.to_string(),
        None => format!("./{normalized}"),
    }
}

/// `path` with `.` left out and `x/..` taken away, with `/` between the parts. Only by the
/// text, without looking at the file system
pub(super) fn normalized(path: &str) -> String {
    let mut parts: Vec<&str> = vec![];
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}
