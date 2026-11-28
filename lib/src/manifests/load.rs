//! Reads manifest files: checks their version, renders their templates and parses them. This
//! is format 1; format 2 files go to the `v2` module

use super::{v2, Manifest, MANIFEST_VERSION};
use crate::{
    contexts::{evaluate_condition, manifest::with_manifest, to_tera, Contexts},
    manifests::get_manifest_name,
    tera_functions::register_functions,
    utilities::error_chain,
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use ignore::{DirEntry, WalkBuilder};
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::canonicalize,
    path::{Path, PathBuf},
};
use tera::Tera;
use thiserror::Error as ThisError;
use tracing::{debug, error, span, warn};

/// Manifests that loaded, and the ones that didn't
#[derive(Debug, Default)]
pub struct Loaded {
    /// The manifests by name
    pub manifests: HashMap<String, Manifest>,
    /// The files that failed
    pub errors: Vec<LoadError>,
}

/// A manifest file that couldn't be loaded
#[derive(Debug, ThisError)]
#[error("Manifest '{name}' ({}) can't be loaded: {kind}", path.display())]
pub struct LoadError {
    /// The manifest's name, as other manifests refer to it in `depends`
    pub name: String,
    /// The file
    pub path: PathBuf,
    /// What went wrong
    pub kind: LoadErrorKind,
}

/// Why a manifest file can't be loaded
#[derive(Debug, ThisError)]
pub enum LoadErrorKind {
    /// The file can't be read
    #[error("the file can't be read: {0}")]
    Read(#[source] std::io::Error),

    /// A template in the file fails
    #[error("templates can't be rendered: {0}")]
    Render(String),

    /// The file isn't a valid manifest
    #[error("{0}")]
    Parse(String),

    /// The file needs a newer comtrya
    #[error("it uses manifest version {found}, but this comtrya only supports version {supported}. Update comtrya")]
    NewerVersion {
        /// The `version` of the file
        found: u32,
        /// The newest version this comtrya supports
        supported: u32,
    },
}

/// The part of a manifest that is readable before its templates are rendered
#[derive(Deserialize)]
struct Unrendered {
    /// The manifest format
    #[serde(default)]
    version: Option<u32>,

    /// The condition, to skip a manifest whose templates fail where it doesn't apply
    #[serde(default)]
    r#where: Option<String>,
}

/// Loads all manifests below `manifest_path`, and logs the ones that can't be loaded
pub fn load(manifest_path: PathBuf, contexts: &Contexts) -> HashMap<String, Manifest> {
    let loaded = load_all(manifest_path, contexts);
    for err in &loaded.errors {
        error!("{err}");
    }
    loaded.manifests
}

/// A glob in `ignore` that can't be read
#[derive(Debug, ThisError)]
#[error("invalid glob `{glob}` in `ignore`: {source}")]
pub struct InvalidGlob {
    /// The glob as written
    pub glob: String,
    /// Why it can't be read
    #[source]
    pub source: globset::Error,
}

/// Paths below a manifest directory that aren't manifests, from `ignore` in the config. The globs
/// match the path relative to the manifest directory, with `/` between directories on every
/// system: `*` stays within one directory, `**` goes into any number of them. A matching
/// directory is left out with everything below it
#[derive(Clone, Debug)]
pub struct Ignore {
    /// The globs, compiled
    globs: GlobSet,
}

impl Default for Ignore {
    fn default() -> Self {
        Self {
            globs: GlobSet::empty(),
        }
    }
}

impl Ignore {
    /// Compiles `globs`
    ///
    /// # Errors
    ///
    /// When a glob can't be read, like `zsh/[a` with an unclosed `[`
    pub fn new<S: AsRef<str>>(globs: &[S]) -> Result<Self, InvalidGlob> {
        let mut builder = GlobSetBuilder::new();
        for glob in globs {
            let glob = glob.as_ref();
            let compiled = GlobBuilder::new(glob)
                .literal_separator(true)
                .build()
                .map_err(|source| InvalidGlob {
                    glob: glob.to_string(),
                    source,
                })?;
            builder.add(compiled);
        }
        let globs = builder.build().map_err(|source| InvalidGlob {
            glob: globs
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join(", "),
            source,
        })?;
        Ok(Self { globs })
    }

    /// Whether `relative`, a path below the manifest directory, is ignored
    fn matches(&self, relative: &Path) -> bool {
        let slashed = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        self.globs.is_match(slashed)
    }
}

/// Loads all manifests below `manifest_path`, and returns the ones that can't be loaded as errors
pub fn load_all(manifest_path: PathBuf, contexts: &Contexts) -> Loaded {
    load_all_ignoring(manifest_path, contexts, &Ignore::default())
}

/// Like [`load_all`], but leaves out the paths that `ignore` matches
pub fn load_all_ignoring(manifest_path: PathBuf, contexts: &Contexts, ignore: &Ignore) -> Loaded {
    let mut loaded = Loaded::default();

    let skipped = SkippedEntries::new(&manifest_path, ignore.clone());
    WalkBuilder::new(&manifest_path)
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        // Arbitrary for now, 9 "should" be enough?
        .max_depth(Some(9))
        .filter_entry(move |entry| !skipped.skips(entry))
        .build()
        .filter(|entry| entry.as_ref().is_ok_and(is_manifest_file))
        .flatten()
        .for_each(|entry| {
            let span = span!(
                tracing::Level::INFO,
                "manifest_load",
                manifest = entry.file_name().to_str()
            )
            .entered();

            // Named by the walked path, which starts with `manifest_path` even when that isn't
            // canonical, like a directory given by a link, or a manifest that is a link
            let name = get_manifest_name(&manifest_path, entry.path())
                .unwrap_or_else(|_| entry.path().display().to_string());
            let path = canonicalize(entry.path()).unwrap_or_else(|_| entry.into_path());
            let dir = path.parent().unwrap_or(&manifest_path);
            let contexts = with_manifest(contexts, &name, dir);

            match load_file(&path, &contexts) {
                Ok(mut manifest) => {
                    manifest.root_dir = path.parent().map(std::path::Path::to_path_buf);
                    manifest.name = Some(name.clone());
                    loaded.manifests.insert(name, manifest);
                }
                Err(kind) => loaded.errors.push(LoadError { name, path, kind }),
            }

            span.exit();
        });

    loaded
}

/// Reads, renders and parses one manifest file
fn load_file(path: &Path, contexts: &Contexts) -> Result<Manifest, LoadErrorKind> {
    let template = std::fs::read_to_string(path).map_err(LoadErrorKind::Read)?;

    // Checked before rendering, a newer format may use templates differently
    let version = parse::<Unrendered>(&template, path)
        .ok()
        .and_then(|unrendered| unrendered.version);
    check_version(version)?;
    if version == Some(v2::VERSION) {
        return v2::load(&template, path, contexts);
    }

    let mut tera = Tera::default();
    register_functions(&mut tera);

    let rendered = match tera.render_str(&template, &to_tera(contexts)) {
        Ok(rendered) => rendered,
        // A template that only works where the manifest applies, like an env var
        // that only exists on Windows, is fine when `where` skips it anyway (#283)
        Err(err) => {
            return match false_where(&template, path, contexts) {
                Some(condition) => {
                    debug!("Not rendering templates, because the 'where' condition is false");
                    Ok(Manifest {
                        r#where: Some(condition),
                        ..Default::default()
                    })
                }
                None => Err(LoadErrorKind::Render(error_chain(&err))),
            }
        }
    };

    let generic = parse::<serde_json::Value>(&rendered, path).ok();
    let unknown = generic
        .as_ref()
        .map(super::unknown_keys)
        .unwrap_or_default();
    let manifest = parse::<Manifest>(&rendered, path).map_err(|err| {
        // A misspelled required field is the likely cause
        let hints: String = unknown.iter().map(|key| format!("; {key}")).collect();
        LoadErrorKind::Parse(format!("{}{hints}", error_chain(&err)))
    })?;
    check_version(manifest.version)?;
    if manifest.version == Some(v2::VERSION) {
        return Err(LoadErrorKind::Parse(String::from(
            "`version: 2` has to be readable before templates are rendered, write it as it is",
        )));
    }
    for key in unknown {
        warn!("{}: {key}", path.display());
    }

    Ok(manifest)
}

/// Why a manifest file isn't valid YAML or TOML
#[derive(Debug, ThisError)]
pub(super) enum ParseError {
    /// The file isn't valid YAML, or doesn't fit the type
    #[error(transparent)]
    Yaml(#[from] serde_yaml_ng::Error),
    /// The file isn't valid TOML, or doesn't fit the type
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    /// The file is neither
    #[error("Unrecognized file extension for manifest")]
    Extension,
}

/// Parses YAML or TOML, by the file extension
pub(super) fn parse<T: DeserializeOwned>(contents: &str, path: &Path) -> Result<T, ParseError> {
    match path.extension().and_then(OsStr::to_str) {
        Some("yaml") | Some("yml") => Ok(serde_yaml_ng::from_str(contents)?),
        Some("toml") => Ok(toml::from_str(contents)?),
        _ => Err(ParseError::Extension),
    }
}

/// Fails for a manifest format newer than this comtrya knows
fn check_version(version: Option<u32>) -> Result<(), LoadErrorKind> {
    match version {
        Some(found) if found > MANIFEST_VERSION => Err(LoadErrorKind::NewerVersion {
            found,
            supported: MANIFEST_VERSION,
        }),
        _ => Ok(()),
    }
}

/// The manifest's `where` condition if it is false, read without rendering templates
fn false_where(contents: &str, path: &Path, contexts: &Contexts) -> Option<String> {
    let condition = parse::<Unrendered>(contents, path).ok()?.r#where?;

    evaluate_condition(&condition, contexts)
        .is_ok_and(|result| !result)
        .then_some(condition)
}

/// What the walk of a manifest directory doesn't go into: `files/` directories, which hold the
/// files that actions copy or link, the config file next to the manifests, and what `ignore`
/// matches
struct SkippedEntries {
    /// The manifest directory, which the `ignore` globs are relative to
    root: PathBuf,
    /// The paths the config file can have, see [`crate::config::FILE_NAMES`]
    configs: Vec<PathBuf>,
    /// The globs from `ignore` in the config
    ignore: Ignore,
}

impl SkippedEntries {
    /// The entries to skip below `manifest_path`
    fn new(manifest_path: &Path, ignore: Ignore) -> Self {
        let configs = crate::config::FILE_NAMES
            .iter()
            .map(|name| manifest_path.join(name))
            .collect();
        Self {
            root: manifest_path.to_path_buf(),
            configs,
            ignore,
        }
    }

    /// Whether the walk leaves out `entry`, and everything below it for a directory
    fn skips(&self, entry: &DirEntry) -> bool {
        let files_dir = entry.file_type().is_some_and(|ft| ft.is_dir())
            && entry.file_name() == OsStr::new("files");
        files_dir
            || self.configs.iter().any(|config| entry.path() == config)
            || self.ignored(entry.path())
    }

    /// Whether `ignore` matches `path`. The manifest directory itself is never ignored
    fn ignored(&self, path: &Path) -> bool {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        let ignored = !relative.as_os_str().is_empty() && self.ignore.matches(relative);
        if ignored {
            debug!("Not a manifest, matches `ignore`: {}", relative.display());
        }
        ignored
    }
}

/// Whether `entry` is a manifest: a `.yaml`, `.yml` or `.toml` file, not a directory
fn is_manifest_file(entry: &DirEntry) -> bool {
    let is_dir = entry.metadata().is_ok_and(|metadata| metadata.is_dir());
    let extension = entry.path().extension().and_then(OsStr::to_str);
    !is_dir && matches!(extension, Some("yaml" | "yml" | "toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::fs;

    /// The names of the manifests that loaded, sorted
    fn names(loaded: &Loaded) -> Vec<String> {
        let mut names: Vec<_> = loaded.manifests.keys().cloned().collect();
        names.sort();
        names
    }

    #[test]
    fn yaml_yml_and_toml_files_are_manifests() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.yaml"), "actions: []\n").unwrap();
        fs::write(dir.path().join("b.yml"), "actions: []\n").unwrap();
        fs::write(dir.path().join("c.toml"), "actions = []\n").unwrap();
        fs::write(dir.path().join("notes.txt"), "not a manifest\n").unwrap();
        fs::create_dir(dir.path().join("d.yaml")).unwrap();

        let loaded = load_all(dir.path().to_path_buf(), &Contexts::default());

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(vec!["a", "b", "c"], names(&loaded));
    }

    #[test]
    fn files_directories_and_the_config_next_to_the_manifests_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.yaml"), "actions: []\n").unwrap();
        for config in crate::config::FILE_NAMES {
            fs::write(dir.path().join(config), "not: [a manifest\n").unwrap();
        }
        fs::create_dir_all(dir.path().join("files")).unwrap();
        fs::write(dir.path().join("files/x.yaml"), "not: [a manifest\n").unwrap();
        fs::create_dir_all(dir.path().join("dev/files")).unwrap();
        fs::write(dir.path().join("dev/files/y.yaml"), "not: [a manifest\n").unwrap();

        let loaded = load_all(dir.path().to_path_buf(), &Contexts::default());

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(vec!["a"], names(&loaded));
    }

    #[test]
    fn a_config_file_deeper_down_is_a_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("dev")).unwrap();
        fs::write(dir.path().join("dev/Comtrya.yaml"), "actions: []\n").unwrap();

        let loaded = load_all(dir.path().to_path_buf(), &Contexts::default());

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(vec!["dev.Comtrya"], names(&loaded));
    }

    #[cfg(unix)]
    #[test]
    fn manifests_are_named_below_a_directory_given_by_a_link() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        fs::create_dir_all(real.join("dev")).unwrap();
        fs::write(real.join("dev/git.yaml"), "actions: []\n").unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let loaded = load_all(link, &Contexts::default());

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        let mut names: Vec<_> = loaded.manifests.keys().cloned().collect();
        names.sort();
        assert_eq!(vec![String::from("dev.git")], names);
    }

    #[cfg(unix)]
    #[test]
    fn a_manifest_that_is_a_link_is_named_by_the_link() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = dir.path().join("elsewhere.yaml");
        fs::write(&elsewhere, "actions: []\n").unwrap();
        let manifests = dir.path().join("manifests");
        fs::create_dir(&manifests).unwrap();
        std::os::unix::fs::symlink(&elsewhere, manifests.join("git.yaml")).unwrap();

        let loaded = load_all(manifests.canonicalize().unwrap(), &Contexts::default());

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert!(
            loaded.manifests.contains_key("git"),
            "{:?}",
            loaded.manifests.keys()
        );
    }

    /// A manifest directory with an app config `zsh/config.yaml` that isn't a manifest, next to
    /// the manifest `zsh/zsh.yaml` and `git.yaml`
    fn with_app_config() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("git.yaml"), "actions: []\n").unwrap();
        fs::create_dir_all(dir.path().join("zsh/plugins")).unwrap();
        fs::write(dir.path().join("zsh/zsh.yaml"), "actions: []\n").unwrap();
        fs::write(dir.path().join("zsh/config.yaml"), "not: [a manifest\n").unwrap();
        fs::write(dir.path().join("zsh/plugins/p.yaml"), "not: [a manifest\n").unwrap();
        dir
    }

    /// Loads the manifests of `dir`, ignoring what matches `globs`
    fn load_ignoring(dir: &Path, globs: &[&str]) -> Loaded {
        let ignore = Ignore::new(globs).unwrap();
        load_all_ignoring(dir.to_path_buf(), &Contexts::default(), &ignore)
    }

    #[test]
    fn an_app_config_below_the_manifests_fails_to_load_without_ignore() {
        let dir = with_app_config();

        let loaded = load_all(dir.path().to_path_buf(), &Contexts::default());

        assert_eq!(2, loaded.errors.len(), "{:?}", loaded.errors);
    }

    #[test]
    fn ignored_paths_are_not_loaded() {
        let dir = with_app_config();

        let loaded = load_ignoring(dir.path(), &["zsh/config.yaml", "zsh/plugins"]);

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(vec!["git", "zsh.zsh"], names(&loaded));
    }

    #[test]
    fn a_double_star_ignores_everything_below_a_directory() {
        let dir = with_app_config();

        let loaded = load_ignoring(dir.path(), &["zsh/**"]);

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(vec!["git"], names(&loaded));
    }

    #[test]
    fn a_single_star_stays_in_its_directory() {
        let dir = with_app_config();

        let loaded = load_ignoring(dir.path(), &["*/config.yaml"]);

        assert_eq!(1, loaded.errors.len(), "{:?}", loaded.errors);
        assert!(loaded.errors[0].path.ends_with("zsh/plugins/p.yaml"));
        assert_eq!(vec!["git", "zsh.zsh"], names(&loaded));
    }

    #[test]
    fn globs_match_below_the_manifest_directory_not_the_whole_path() {
        let dir = with_app_config();
        let absolute = format!("{}/**", dir.path().display());

        let loaded = load_ignoring(dir.path(), &[absolute.as_str(), "**/plugins"]);

        assert_eq!(1, loaded.errors.len(), "{:?}", loaded.errors);
        assert_eq!(vec!["git", "zsh.zsh"], names(&loaded));
    }

    #[test]
    fn an_invalid_glob_is_an_error() {
        let err = Ignore::new(&["zsh/[a"]).unwrap_err();

        assert!(err.to_string().contains("zsh/[a"), "{err}");
    }
}
