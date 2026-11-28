//! The `manifest` context: the name and the directory of the manifest that is loaded or runs.
//! Unlike the other contexts it differs per manifest, so `comtrya contexts` doesn't list it

use super::Contexts;
use crate::manifests::Manifest;
use std::collections::BTreeMap;
use std::path::Path;

/// `contexts` plus `manifest.name` and `manifest.dir` for the manifest `name` in `dir`
pub fn with_manifest(contexts: &Contexts, name: &str, dir: &Path) -> Contexts {
    let mut contexts = contexts.clone();
    contexts.insert(
        String::from("manifest"),
        BTreeMap::from([
            (String::from("name"), name.to_string().into()),
            (
                String::from("dir"),
                super::user::template_path(&dir.display().to_string()).into(),
            ),
        ]),
    );
    contexts
}

/// `contexts` plus the `manifest` context of `manifest`, if it has a name and a directory
pub fn for_manifest(contexts: &Contexts, manifest: &Manifest) -> Contexts {
    match (&manifest.name, &manifest.root_dir) {
        (Some(name), Some(dir)) => with_manifest(contexts, name, dir),
        _ => contexts.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_manifest_context_has_its_name_and_directory() {
        let contexts = with_manifest(&Contexts::new(), "dev.git", Path::new("/dots/dev"));

        assert_eq!("dev.git", contexts["manifest"]["name"].to_string());
        assert_eq!("/dots/dev", contexts["manifest"]["dir"].to_string());
    }

    #[test]
    fn a_manifest_without_name_or_directory_adds_nothing() {
        let manifest = Manifest {
            root_dir: Some(PathBuf::from("/dots")),
            ..Default::default()
        };

        assert!(!for_manifest(&Contexts::new(), &manifest).contains_key("manifest"));
    }
}
