//! Black-box tests for `comtrya apply` and `comtrya status`: real manifests in a tempdir, run
//! through the binary. They pin behaviour users rely on, independent of how it's implemented.

use assert_cmd::{assert::Assert, Command};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Fixture {
            dir: TempDir::new().expect("could not create tempdir"),
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Writes a manifest below `manifests/`
    fn manifest(&self, name: &str, contents: &str) -> &Self {
        let path = self.path().join("manifests").join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
        self
    }

    /// A `directory.create` action for `<tempdir>/out/<name>`, to observe that a manifest ran
    fn create(&self, name: &str) -> String {
        format!(
            "  - action: directory.create\n    path: '{}'\n",
            self.out(name).display()
        )
    }

    fn out(&self, name: &str) -> PathBuf {
        self.path().join("out").join(name)
    }

    fn created(&self, name: &str) -> bool {
        self.out(name).is_dir()
    }

    /// Runs comtrya isolated from the user's own Comtrya.yaml and config directory
    fn command(&self, args: &[&str]) -> Command {
        let manifests = self.path().join("manifests");
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("comtrya"));
        cmd.current_dir(self.path())
            .env("HOME", self.path())
            .env("XDG_CONFIG_HOME", self.path())
            .args(["--no-color", "-d"])
            .arg(manifests)
            .args(args);
        cmd
    }

    fn run(&self, args: &[&str]) -> Assert {
        self.command(args).assert().success()
    }

    /// Like `run`, but a manifest is expected to fail, so comtrya must exit with an error
    fn run_failing(&self, args: &[&str]) -> Assert {
        self.command(args).assert().failure()
    }
}

#[test]
fn applies_all_manifests() {
    let f = Fixture::new();
    f.manifest("one.yaml", &format!("actions:\n{}", f.create("one")))
        .manifest("nested/two.yaml", &format!("actions:\n{}", f.create("two")));

    f.run(&["apply"]);

    assert!(f.created("one"));
    assert!(f.created("two"));
}

#[test]
fn toml_manifests_are_applied() {
    let f = Fixture::new();
    f.manifest(
        "one.toml",
        &format!(
            "[[actions]]\naction = \"directory.create\"\npath = '{}'\n",
            f.out("one").display()
        ),
    );

    f.run(&["apply"]);

    assert!(f.created("one"));
}

#[test]
fn dry_run_changes_nothing() {
    let f = Fixture::new();
    f.manifest("one.yaml", &format!("actions:\n{}", f.create("one")));

    f.run(&["apply", "--dry-run"]);

    assert!(!f.created("one"));
}

#[test]
fn selected_manifests_run_with_their_dependencies_only() {
    let f = Fixture::new();
    f.manifest(
        "app.yaml",
        &format!("depends:\n  - lib\nactions:\n{}", f.create("app")),
    )
    .manifest("lib.yaml", &format!("actions:\n{}", f.create("lib")))
    .manifest("other.yaml", &format!("actions:\n{}", f.create("other")));

    f.run(&["apply", "-m", "app"]);

    assert!(f.created("app"));
    assert!(f.created("lib"));
    assert!(!f.created("other"));
}

#[test]
fn nested_manifests_are_selected_by_dotted_name() {
    let f = Fixture::new();
    f.manifest(
        "dev/tools.yaml",
        &format!("actions:\n{}", f.create("tools")),
    )
    .manifest("dev/main.yaml", &format!("actions:\n{}", f.create("dev")))
    .manifest("other.yaml", &format!("actions:\n{}", f.create("other")));

    // `dev/main.yaml` is called `dev`, `dev/tools.yaml` is `dev.tools`
    f.run(&["apply", "-m", "dev.tools,dev"]);

    assert!(f.created("tools"));
    assert!(f.created("dev"));
    assert!(!f.created("other"));
}

#[test]
fn where_condition_skips_manifest() {
    let f = Fixture::new();
    f.manifest(
        "skipped.yaml",
        &format!(
            "where: os.name == \"no-such-os\"\nactions:\n{}",
            f.create("skipped")
        ),
    )
    .manifest(
        "applied.yaml",
        &format!(
            "where: os.name != \"no-such-os\"\nactions:\n{}",
            f.create("applied")
        ),
    );

    f.run(&["apply"]);

    assert!(!f.created("skipped"));
    assert!(f.created("applied"));
}

#[test]
fn where_condition_sees_defined_variables() {
    let f = Fixture::new();
    f.manifest(
        "work.yaml",
        &format!(
            "where: variables.profile == \"work\"\nactions:\n{}",
            f.create("work")
        ),
    );

    f.run(&["apply"]);
    assert!(!f.created("work"));

    f.run(&["-D", "profile=work", "apply"]);
    assert!(f.created("work"));
}

/// Templates of a manifest that `where` skips may fail, e.g. on a Windows-only env var (#283)
#[test]
fn where_condition_skips_manifest_before_rendering_templates() {
    let f = Fixture::new();
    let windows_only = |condition: &str| {
        format!(
            "where: {condition}\nactions:\n  - action: directory.create\n    path: '{{{{ get_env(name=\"COMTRYA_UNSET_VARIABLE\") }}}}'\n"
        )
    };
    f.manifest("skipped.yaml", &windows_only("os.name == \"no-such-os\""))
        .manifest(
            "applied.yaml",
            &format!("actions:\n{}", f.create("applied")),
        );

    let output = f.run(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(f.created("applied"));
    assert!(!log.contains("ERROR"), "{log}");

    // Where the manifest applies, the broken template is still reported
    f.manifest("skipped.yaml", &windows_only("os.name != \"no-such-os\""));

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(log.contains("COMTRYA_UNSET_VARIABLE"), "{log}");
}

/// A manifest for a newer format is refused instead of misread, and fails the run
#[test]
fn manifests_with_a_newer_version_are_skipped() {
    let f = Fixture::new();
    f.manifest(
        "newer.yaml",
        &format!("version: 2\nactions:\n{}", f.create("newer")),
    )
    .manifest(
        "current.yaml",
        &format!("version: 1\nactions:\n{}", f.create("current")),
    );

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(f.created("current"));
    assert!(!f.created("newer"));
    assert!(log.contains("manifest version 2"), "{log}");
}

#[test]
fn templates_are_rendered_with_variables() {
    let f = Fixture::new();
    f.manifest(
        "one.yaml",
        "actions:\n  - action: directory.create\n    path: '{{ variables.base }}/{{ variables.name }}'\n",
    );

    let base = f.path().join("out").display().to_string();
    let base = format!("base={base}");
    f.run(&["-D", &base, "-D", "name=rendered", "apply"]);

    assert!(f.created("rendered"));
}

#[test]
fn label_selects_manifests() {
    let f = Fixture::new();
    f.manifest(
        "desktop.yaml",
        &format!("labels:\n  - desktop\nactions:\n{}", f.create("desktop")),
    )
    .manifest(
        "server.yaml",
        &format!("labels:\n  - server\nactions:\n{}", f.create("server")),
    )
    .manifest(
        "unlabeled.yaml",
        &format!("actions:\n{}", f.create("unlabeled")),
    );

    f.run(&["apply", "-l", "desktop"]);

    assert!(f.created("desktop"));
    assert!(!f.created("server"));
    assert!(!f.created("unlabeled"));
}

#[test]
fn broken_manifests_dont_stop_the_others() {
    let f = Fixture::new();
    f.manifest("invalid.yaml", "actions:\n  - action: no.such.action\n")
        .manifest(
            "unknown-variable.yaml",
            &format!(
                "actions:\n  - action: directory.create\n    path: '{{{{ variables.missing }}}}'\n{}",
                f.create("never")
            ),
        )
        .manifest("valid.yaml", &format!("actions:\n{}", f.create("valid")));

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(f.created("valid"));
    assert!(!f.created("never"));
    assert!(log.contains("Manifest 'invalid'"), "{log}");
    assert!(log.contains("Manifest 'unknown-variable'"), "{log}");
}

/// A manifest that depends on one that can't be loaded must not run without it
#[test]
fn manifests_that_depend_on_a_broken_one_are_skipped() {
    let f = Fixture::new();
    f.manifest("broken.yaml", "actions:\n  - action: no.such.action\n")
        .manifest(
            "dependent.yaml",
            &format!("depends: [broken]\nactions:\n{}", f.create("dependent")),
        )
        .manifest(
            "missing.yaml",
            &format!(
                "depends: [no-such-manifest]\nactions:\n{}",
                f.create("missing")
            ),
        )
        .manifest(
            "unrelated.yaml",
            &format!("actions:\n{}", f.create("unrelated")),
        );

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(f.created("unrelated"));
    assert!(!f.created("dependent"));
    assert!(!f.created("missing"));
    assert!(log.contains("no-such-manifest"), "{log}");
}

/// Selecting a broken manifest with `-m` fails too, instead of doing nothing
#[test]
fn selecting_a_broken_manifest_fails() {
    let f = Fixture::new();
    f.manifest("broken.yaml", "actions:\n  - action: no.such.action\n");

    f.run_failing(&["apply", "-m", "broken"]);
}

/// `--dry-run` is how CI checks manifests, so broken ones must fail it
#[test]
fn dry_run_fails_on_broken_manifests() {
    let f = Fixture::new();
    f.manifest("broken.yaml", "actions:\n  - action: no.such.action\n")
        .manifest("valid.yaml", &format!("actions:\n{}", f.create("valid")));

    f.run_failing(&["apply", "--dry-run"]);
    f.run_failing(&["status"]);

    assert!(!f.created("valid"));
}

/// A `file.copy` whose source is missing, which fails when the action is planned
fn copy_missing(f: &Fixture, name: &str) -> String {
    format!(
        "  - action: file.copy\n    from: {name}\n    to: '{}'\n",
        f.out(name).display()
    )
}

/// Actions that can't be planned fail a dry run too, like they fail a real one
#[test]
fn dry_run_fails_when_an_action_cant_be_planned() {
    let f = Fixture::new();
    f.manifest(
        "broken.yaml",
        &format!("actions:\n{}", copy_missing(&f, "zshrc")),
    )
    .manifest("valid.yaml", &format!("actions:\n{}", f.create("valid")));

    let output = f.run_failing(&["apply", "--dry-run"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(log.contains("zshrc was not found"), "{log}");
    assert!(log.contains("1 manifest(s) failed"), "{log}");
    assert!(!f.created("valid"));
}

/// A dry run changes nothing, so it reports every action of a manifest that can't be planned
#[test]
fn dry_run_reports_every_action_that_cant_be_planned() {
    let f = Fixture::new();
    f.manifest(
        "broken.yaml",
        &format!(
            "actions:\n{}{}{}",
            copy_missing(&f, "zshrc"),
            f.create("between"),
            copy_missing(&f, "bashrc")
        ),
    );

    let stdout = f
        .run_failing(&["apply", "--dry-run"])
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&stdout);

    assert!(stdout.contains("zshrc was not found"), "{stdout}");
    assert!(stdout.contains("bashrc was not found"), "{stdout}");
}

#[test]
fn dry_run_skips_dependents_of_an_action_that_cant_be_planned() {
    let f = Fixture::new();
    f.manifest(
        "broken.yaml",
        &format!("actions:\n{}", copy_missing(&f, "zshrc")),
    )
    .manifest(
        "dependent.yaml",
        &format!("depends: [broken]\nactions:\n{}", f.create("dependent")),
    );

    let output = f.run_failing(&["apply", "--dry-run"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);

    assert!(
        log.contains("Skipping manifest, because a dependency failed"),
        "{log}"
    );
    assert!(log.contains("2 manifest(s) failed"), "{log}");
}

/// Output of a failing run, to check that it explains the error instead of panicking
fn failure_log(f: &Fixture, args: &[&str]) -> String {
    let output = f.run_failing(args).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    assert!(!log.contains("panicked"), "{log}");
    log.to_string()
}

#[test]
fn a_missing_manifest_directory_is_reported() {
    let f = Fixture::new();

    let log = failure_log(&f, &["apply"]);

    assert!(log.contains("Can't find the manifests at"), "{log}");
    assert!(log.contains("doesn't exist or can't be read"), "{log}");
}

#[test]
fn a_missing_directory_copy_source_is_reported() {
    let f = Fixture::new();
    f.manifest(
        "dir.yaml",
        &format!(
            "actions:\n  - action: directory.copy\n    from: nope\n    to: '{}'\n",
            f.out("copy").display()
        ),
    );

    let log = failure_log(&f, &["apply"]);

    assert!(log.contains("nope was not found"), "{log}");
}

#[test]
fn file_link_without_source_is_reported() {
    let f = Fixture::new();
    f.manifest(
        "link.yaml",
        &format!(
            "actions:\n  - action: file.link\n    target: '{}'\n",
            f.out("link").display()
        ),
    );

    let log = failure_log(&f, &["apply"]);

    assert!(log.contains("file.link needs `source`"), "{log}");
}

/// YAML and TOML files in `files` directories are files for actions, never manifests (#575)
#[test]
fn files_directories_are_not_loaded_as_manifests() {
    let f = Fixture::new();
    let leak = format!("actions:\n{}", f.create("leaked"));
    let leak_toml = format!(
        "[[actions]]\naction = \"directory.create\"\npath = '{}'\n",
        f.out("leaked-toml").display()
    );
    f.manifest("app/main.yaml", &format!("actions:\n{}", f.create("app")))
        .manifest("app/files/config.yaml", &leak)
        .manifest("app/files/nested/deep.yml", &leak)
        .manifest("app/files/settings.toml", &leak_toml)
        .manifest("files/top-level.yaml", &leak);

    f.run(&["apply"]);

    assert!(f.created("app"));
    assert!(!f.created("leaked"));
    assert!(!f.created("leaked-toml"));

    let output = f.run(&["status"]).get_output().stdout.clone();
    let status = String::from_utf8_lossy(&output);
    assert!(status.contains("| app "), "{status}");
    assert!(!status.contains("files"), "{status}");
}

#[test]
fn user_directories_work_in_double_quoted_strings() {
    // On Windows, `C:\Users\...` in a double-quoted YAML string is an invalid escape
    let f = Fixture::new();
    f.manifest(
        "dirs.yaml",
        &format!(
            "actions:\n  - action: directory.create\n    path: \"{{{{ user.home_dir }}}}\"\n{}",
            f.create("ran")
        ),
    );

    f.run(&["apply"]);

    assert!(f.created("ran"));
}

/// Without a `files` directory, the error names the missing file instead of panicking (#246)
#[test]
fn a_missing_source_file_is_reported() {
    let f = Fixture::new();
    f.manifest(
        "zsh.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: zshrc\n    to: '{}'\n",
            f.out("zshrc").display()
        ),
    );

    let output = f.run_failing(&["apply"]).get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let error = stdout
        .lines()
        .find(|line| line.contains("zshrc was not found"))
        .unwrap_or_else(|| panic!("missing file not named in:\n{stdout}"));
    assert!(error.contains("ERROR"), "{error}");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
}

#[test]
fn file_copy_renders_templates() {
    let f = Fixture::new();
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: gitconfig\n    to: '{}'\n    template: true\n",
            f.out("gitconfig").display()
        ),
    )
    .manifest("dotfiles/files/gitconfig", "name = {{ variables.name }}\n");
    std::fs::create_dir_all(f.path().join("out")).unwrap();

    f.run(&["-D", "name=Comtrya", "apply"]);

    assert_eq!(
        "name = Comtrya\n",
        std::fs::read_to_string(f.out("gitconfig")).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn file_copy_processes_contents_with_a_command() {
    let f = Fixture::new();
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: greeting\n    to: '{}'\n    process_with: [tr, h, H]\n    template: true\n",
            f.out("greeting").display()
        ),
    )
    .manifest("dotfiles/files/greeting", "hello {{ variables.name }}\n");
    std::fs::create_dir_all(f.path().join("out")).unwrap();

    f.run(&["-D", "name=Comtrya", "apply"]);

    // The command runs first, so the template is rendered from its output
    assert_eq!(
        "Hello Comtrya\n",
        std::fs::read_to_string(f.out("greeting")).unwrap()
    );
}

/// `--dry-run` shows what `file.copy` would change in an existing file (#593)
#[test]
fn file_copy_dry_run_shows_a_diff() {
    let f = Fixture::new();
    let to = f.out("config");
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: config\n    to: '{}'\n",
            to.display()
        ),
    )
    .manifest("dotfiles/files/config", "one\nnew line\nthree\n");
    std::fs::create_dir_all(f.path().join("out")).unwrap();
    std::fs::write(&to, "one\nold line\nthree\n").unwrap();

    let output = f.run(&["apply", "--dry-run"]).get_output().stdout.clone();
    let log = String::from_utf8_lossy(&output);

    assert!(log.contains("-old line"), "{log}");
    assert!(log.contains("+new line"), "{log}");
    assert_eq!(
        "one\nold line\nthree\n",
        std::fs::read_to_string(&to).unwrap()
    );
}

/// Contents produced by `process_with`, e.g. decrypted secrets, never show up in a diff
#[cfg(unix)]
#[test]
fn file_copy_dry_run_does_not_show_processed_contents() {
    let f = Fixture::new();
    let to = f.out("secret");
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: secret\n    to: '{}'\n    process_with: [tr, a-z, A-Z]\n",
            to.display()
        ),
    )
    .manifest("dotfiles/files/secret", "new-password\n");
    std::fs::create_dir_all(f.path().join("out")).unwrap();
    std::fs::write(&to, "OLD-PASSWORD\n").unwrap();

    let output = f.run(&["apply", "--dry-run"]).get_output().stdout.clone();
    let log = String::from_utf8_lossy(&output);

    assert!(log.contains("not shown"), "{log}");
    assert!(!log.contains("PASSWORD"), "{log}");
}

/// Decrypts a file with the throwaway key in `tests/fixtures/gpg`, in its own keyring.
/// Skipped when gpg isn't installed, except in CI, where it must run.
#[cfg(unix)]
#[test]
fn file_copy_decrypts_with_gpg() {
    use std::process::Command as Process;

    if Process::new("gpg").arg("--version").output().is_err() {
        assert!(
            std::env::var_os("CI").is_none(),
            "gpg must be installed in CI"
        );
        eprintln!("gpg not found, skipping");
        return;
    }

    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gpg");
    // gpg-agent's socket lives in GNUPGHOME, and socket paths must stay short
    let gnupg_home = tempfile::Builder::new()
        .prefix("gpg")
        .tempdir_in("/tmp")
        .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(gnupg_home.path(), std::fs::Permissions::from_mode(0o700))
            .unwrap();
    }
    let gpg = |args: &[&str]| {
        let status = Process::new("gpg")
            .env("GNUPGHOME", gnupg_home.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "gpg {args:?} failed");
    };
    gpg(&[
        "--batch",
        "--quiet",
        "--import",
        fixtures.join("secret-key.asc").to_str().unwrap(),
    ]);

    let f = Fixture::new();
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.copy\n    from: credentials.fish.asc\n    to: '{}'\n    chmod: 600\n    process_with: [gpg, --batch, --quiet, --decrypt]\n",
            f.out("credentials.fish").display()
        ),
    );
    let files = f.path().join("manifests/dotfiles/files");
    std::fs::create_dir_all(&files).unwrap();
    std::fs::copy(
        fixtures.join("credentials.fish.asc"),
        files.join("credentials.fish.asc"),
    )
    .unwrap();
    std::fs::create_dir_all(f.path().join("out")).unwrap();

    let result = f
        .command(&["apply"])
        .env("GNUPGHOME", gnupg_home.path())
        .assert();
    let _ = Process::new("gpgconf")
        .env("GNUPGHOME", gnupg_home.path())
        .args(["--kill", "gpg-agent"])
        .status();
    result.success();

    let target = f.out("credentials.fish");
    assert_eq!(
        "set -gx API_TOKEN not-a-real-token\n",
        std::fs::read_to_string(&target).unwrap()
    );
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        0o600,
        std::fs::metadata(&target).unwrap().permissions().mode() & 0o777
    );
}

#[cfg(unix)]
#[test]
fn file_link_links_into_files_directory() {
    let f = Fixture::new();
    f.manifest(
        "dotfiles/main.yaml",
        &format!(
            "actions:\n  - action: file.link\n    from: vimrc\n    to: '{}'\n",
            f.out("vimrc").display()
        ),
    )
    .manifest("dotfiles/files/vimrc", "set number\n");
    std::fs::create_dir_all(f.path().join("out")).unwrap();

    f.run(&["apply"]);

    let target = std::fs::read_link(f.out("vimrc")).unwrap();
    assert!(target.ends_with("dotfiles/files/vimrc"), "{target:?}");
    assert_eq!(
        "set number\n",
        std::fs::read_to_string(f.out("vimrc")).unwrap()
    );
}

#[test]
fn contexts_lists_builtin_contexts() {
    let f = Fixture::new();
    std::fs::create_dir_all(f.path().join("manifests")).unwrap();

    let output = f.run(&["contexts"]).get_output().stdout.clone();
    let output = String::from_utf8_lossy(&output);

    for context in ["env", "os", "user", "variables"] {
        assert!(
            output.lines().any(|line| line.trim() == context),
            "{context} missing"
        );
    }
}

/// `stdout` writes the command's output to a file, relative to `dir` (#117)
#[cfg(unix)]
#[test]
fn command_run_writes_stdout_to_a_file() {
    let f = Fixture::new();
    f.manifest(
        "completions.yaml",
        &format!(
            "actions:\n  - action: command.run\n    command: echo\n    args:\n      - complete -c comtrya\n    dir: '{}'\n    stdout: out/completions/comtrya\n",
            f.path().display()
        ),
    );

    f.run(&["apply"]);

    assert_eq!(
        "complete -c comtrya\n",
        std::fs::read_to_string(f.out("completions").join("comtrya")).unwrap()
    );
}

/// `env` only applies to its command. It used to be set on comtrya itself and removed
/// afterwards, which left later commands without a PATH
#[cfg(unix)]
#[test]
fn command_run_env_does_not_leak_into_later_actions() {
    let f = Fixture::new();
    f.manifest(
        "env.yaml",
        &format!(
            "actions:\n  - action: command.run\n    command: sh\n    args: ['-c', 'test \"$GREETING\" = hi']\n    env:\n      GREETING: hi\n      PATH: /usr/bin:/bin\n  - action: command.run\n    command: sh\n    args: ['-c', 'test -z \"$GREETING\"']\n{}",
            f.create("after")
        ),
    );

    f.run(&["apply"]);

    assert!(f.created("after"));
}

/// `op://` in include_variables, with a fake 1Password CLI on PATH instead of a real vault (#306)
#[cfg(unix)]
mod one_password {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    /// Installs a fake `op` running `script`, and a Comtrya.yaml including `include`
    fn setup(f: &Fixture, script: &str, include: &str) -> (String, PathBuf) {
        let bin = f.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let op = bin.join("op");
        std::fs::write(&op, format!("#!/bin/sh\n{script}\n")).unwrap();
        std::fs::set_permissions(&op, std::fs::Permissions::from_mode(0o755)).unwrap();

        let config = f.path().join("Comtrya.yaml");
        std::fs::write(&config, format!("include_variables:\n  - '{include}'\n")).unwrap();

        let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
        (path, config)
    }

    fn log(output: &std::process::Output) -> String {
        String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr)
    }

    #[test]
    fn item_fields_become_variables() {
        let f = Fixture::new();
        let script = format!(
            "echo \"$@\" > '{}'\necho '{{\"fields\": [{{\"label\": \"folder\", \"value\": \"from-op\"}}]}}'",
            f.path().join("args").display()
        );
        let (path, config) = setup(&f, &script, "op://My Vault/Laptop?account=my.1password.com");
        f.manifest(
            "secret.yaml",
            &format!(
                "actions:\n  - action: directory.create\n    path: '{}/{{{{ include_variables.folder }}}}'\n",
                f.path().join("out").display()
            ),
        );

        let mut cmd = f.command(&["-c", config.to_str().unwrap(), "apply"]);
        let output = cmd.env("PATH", &path).output().unwrap();

        assert!(output.status.success(), "{}", log(&output));
        assert!(f.created("from-op"), "{}", log(&output));

        // `comtrya contexts` lists the key, but keeps the value to itself
        let mut cmd = f.command(&["-c", config.to_str().unwrap(), "contexts"]);
        let output = cmd.env("PATH", &path).output().unwrap();
        let contexts = log(&output);
        assert!(contexts.contains("folder"), "{contexts}");
        assert!(!contexts.contains("from-op"), "value shown: {contexts}");

        let args = std::fs::read_to_string(f.path().join("args")).unwrap();
        assert_eq!(
            args.trim(),
            "item get Laptop --vault My Vault --format json --account my.1password.com"
        );
    }

    #[test]
    fn op_errors_are_reported() {
        let f = Fixture::new();
        let (path, config) = setup(
            &f,
            "echo '[ERROR] You are not currently signed in.' >&2\nexit 1",
            "op://Private/GitHub",
        );
        std::fs::create_dir_all(f.path().join("manifests")).unwrap();

        let mut cmd = f.command(&["-c", config.to_str().unwrap(), "contexts"]);
        let output = cmd.env("PATH", path).output().unwrap();

        let log = log(&output);
        assert!(log.contains("You are not currently signed in"), "{log}");
        assert!(log.contains("op signin"), "{log}");
    }
}

/// `package_provider` in Comtrya.yaml replaces the OS default; `provider` on an action wins (#456).
/// zypper and xbps have no bootstrap, so planning fails before anything is installed, with a
/// predictable message on CI runners
#[test]
fn package_provider_can_be_configured() {
    let f = Fixture::new();
    std::fs::write(
        f.path().join("Comtrya.yaml"),
        "disable_update_check: true\npackage_provider: zypper\n",
    )
    .unwrap();
    f.manifest(
        "configured.yaml",
        "actions:\n  - action: package.install\n    name: curl\n",
    );

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    assert!(log.contains("Zypper, isn't available"), "{log}");

    f.manifest(
        "configured.yaml",
        "actions:\n  - action: package.install\n    name: curl\n    provider: xbps\n",
    );

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    assert!(log.contains("Xbps, isn't available"), "{log}");
    assert!(!log.contains("Zypper"), "{log}");

    // `package` is reserved in Rhai, so the context is `packages`
    std::fs::remove_file(f.path().join("manifests/configured.yaml")).unwrap();
    f.manifest(
        "where.yaml",
        &format!(
            "where: packages.provider == \"zypper\"\nactions:\n{}",
            f.create("{{ packages.provider }}")
        ),
    );

    f.run(&["apply"]);
    assert!(f.created("zypper"));
}

/// An unknown provider name in Comtrya.yaml is an error that names the valid ones
#[test]
fn an_unknown_package_provider_is_reported() {
    let f = Fixture::new();
    std::fs::write(f.path().join("Comtrya.yaml"), "package_provider: nope\n").unwrap();
    std::fs::create_dir_all(f.path().join("manifests")).unwrap();

    let output = f.run_failing(&["apply"]).get_output().clone();
    let log = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    assert!(log.contains("unknown variant `nope`"), "{log}");
}

#[cfg(unix)]
mod order {
    use super::*;

    /// A `command.run` action appending `name` to `<tempdir>/log`
    fn log(f: &Fixture, name: &str) -> String {
        format!(
            "  - action: command.run\n    command: sh\n    args:\n      - -c\n      - echo {name} >> '{}'\n",
            f.path().join("log").display()
        )
    }

    fn logged(f: &Fixture) -> Vec<String> {
        std::fs::read_to_string(f.path().join("log"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    #[test]
    fn dependencies_run_first() {
        let f = Fixture::new();
        f.manifest(
            "app.yaml",
            &format!("depends:\n  - lib\nactions:\n{}", log(&f, "app")),
        )
        .manifest(
            "lib.yaml",
            &format!("depends:\n  - base\nactions:\n{}", log(&f, "lib")),
        )
        .manifest("base.yaml", &format!("actions:\n{}", log(&f, "base")));

        f.run(&["apply", "-m", "app"]);

        assert_eq!(vec!["base", "lib", "app"], logged(&f));
    }

    #[test]
    fn relative_dependencies_resolve_in_the_same_directory() {
        let f = Fixture::new();
        f.manifest(
            "dev/app.yaml",
            &format!("depends:\n  - ./lib\nactions:\n{}", log(&f, "app")),
        )
        .manifest("dev/lib.yaml", &format!("actions:\n{}", log(&f, "dev.lib")))
        .manifest("lib.yaml", &format!("actions:\n{}", log(&f, "lib")));

        f.run(&["apply", "-m", "dev.app"]);

        assert_eq!(vec!["dev.lib", "app"], logged(&f));
    }

    #[test]
    fn actions_run_in_order() {
        let f = Fixture::new();
        f.manifest(
            "one.yaml",
            &format!("actions:\n{}{}{}", log(&f, "1"), log(&f, "2"), log(&f, "3")),
        );

        f.run(&["apply"]);

        assert_eq!(vec!["1", "2", "3"], logged(&f));
    }

    #[test]
    fn dependents_dont_run_when_a_dependency_fails() {
        let f = Fixture::new();
        f.manifest(
            "app.yaml",
            &format!("depends:\n  - lib\nactions:\n{}", log(&f, "app")),
        )
        .manifest(
            "lib.yaml",
            &format!(
                "actions:\n{}  - action: command.run\n    command: 'false'\n",
                log(&f, "lib")
            ),
        );

        f.run_failing(&["apply", "-m", "app"]);

        assert_eq!(vec!["lib"], logged(&f));
    }

    fn fail() -> &'static str {
        "  - action: command.run\n    command: 'false'\n"
    }

    #[test]
    fn a_failing_command_reports_why() {
        let f = Fixture::new();
        f.manifest(
            "broken.yaml",
            "actions:\n  - action: command.run\n    command: sh\n    args:\n      - -c\n      - echo something broke >&2; exit 3\n",
        );

        let output = f.run_failing(&["apply"]).get_output().clone();
        let stdout = String::from_utf8_lossy(&output.stdout);

        let error = stdout
            .lines()
            .find(|line| line.contains("ERROR") && line.contains("exit code"))
            .unwrap_or_else(|| panic!("no error with the exit code in:\n{stdout}"));
        assert!(error.contains("exit code: 3"), "{error}");
        assert!(error.contains("something broke"), "{error}");
    }

    #[test]
    fn remaining_actions_of_a_failing_manifest_are_skipped() {
        let f = Fixture::new();
        f.manifest(
            "one.yaml",
            &format!("actions:\n{}{}{}", log(&f, "1"), fail(), log(&f, "2")),
        );

        f.run_failing(&["apply"]);

        assert_eq!(vec!["1"], logged(&f));
    }

    #[test]
    fn unrelated_manifests_run_when_one_fails() {
        let f = Fixture::new();
        f.manifest("broken.yaml", &format!("actions:\n{}", fail()));
        // Manifests run in no particular order, so use several to make it unlikely that all of
        // them happen to run before the failing one
        let others: Vec<String> = (0..9).map(|i| format!("other{i}")).collect();
        for other in &others {
            f.manifest(
                &format!("{other}.yaml"),
                &format!("actions:\n{}", log(&f, other)),
            );
        }

        f.run_failing(&["apply"]);

        let mut ran = logged(&f);
        ran.sort();
        assert_eq!(others, ran);
    }

    #[test]
    fn dependents_are_skipped_transitively() {
        let f = Fixture::new();
        f.manifest(
            "app.yaml",
            &format!("depends:\n  - lib\nactions:\n{}", log(&f, "app")),
        )
        .manifest(
            "lib.yaml",
            &format!("depends:\n  - base\nactions:\n{}", log(&f, "lib")),
        )
        .manifest("base.yaml", &format!("actions:\n{}", fail()))
        .manifest(
            "tool.yaml",
            &format!("depends:\n  - shared\nactions:\n{}", log(&f, "tool")),
        )
        .manifest("shared.yaml", &format!("actions:\n{}", log(&f, "shared")));

        f.run_failing(&["apply"]);

        let mut ran = logged(&f);
        ran.sort();
        assert_eq!(vec!["shared", "tool"], ran);
    }
}

/// Examples that fail to load, e.g. only on one platform
const KNOWN_FAILURES: [&str; 0] = [];

/// Every example in the repository must load. Catches breaking changes to the manifest format.
#[test]
fn all_examples_load() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let home = TempDir::new().unwrap();

    // Offline replacement for examples/Comtrya.yaml, whose include_variables needs DNS
    let config = home.path().join("Comtrya.yaml");
    std::fs::write(
        &config,
        "variables:\n  fkbr: sxoe\n  kuci: xhpv\n  hello: hello\n  foo: foo\n",
    )
    .unwrap();

    let output = Command::new(assert_cmd::cargo::cargo_bin!("comtrya"))
        // Some examples read files relative to the repository root
        .current_dir(repo)
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("AGE_PASSPHRASE", "not-a-secret")
        .args(["--no-color", "-c"])
        .arg(&config)
        .args(["-d", "examples", "status"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let errors: Vec<&str> = stdout.lines().filter(|l| l.contains("ERROR")).collect();
    let failing = errors.len();
    for known in KNOWN_FAILURES {
        assert!(
            errors.iter().any(|error| error.contains(known)),
            "{known} loads now, remove it from KNOWN_FAILURES"
        );
    }
    assert_eq!(KNOWN_FAILURES.len(), failing, "{stdout}");

    // One table row per manifest file. Long names wrap, so only count rows with an action count
    let manifest_files = walkdir::WalkDir::new(repo.join("examples"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            let path = e.path();
            path.extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml" || ext == "toml")
                && path.file_name().is_some_and(|name| name != "Comtrya.yaml")
                && !path.components().any(|c| c.as_os_str() == "files")
        })
        .count();
    let rows = stdout
        .lines()
        .filter(|line| {
            let cells: Vec<&str> = line.split('|').collect();
            cells.len() > 2
                && cells[2].trim().parse::<usize>().is_ok()
                // FIXME: the Comtrya.yaml config is loaded as a manifest as well
                && cells[1].trim() != "Comtrya"
        })
        .count();

    assert!(manifest_files > 20, "only {manifest_files} examples found");
    assert_eq!(manifest_files - failing, rows, "{stdout}");
}
