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
    #[cfg(unix)]
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

    f.run(&["apply"]);

    assert!(f.created("valid"));
    assert!(!f.created("never"));
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
