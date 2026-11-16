//! Project tasks, run with `cargo xtask <task>`. `check` runs what CI runs, so a pull request
//! passes CI when it passes here

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Printed for no task or an unknown one
const USAGE: &str = "usage: cargo xtask check [--fix]

check   formats, lints, tests and builds the docs like CI, then checks the MSRV
        --fix  formats the code instead of failing on unformatted code";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["check"] => check(false),
        ["check", "--fix"] => check(true),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// One cargo call of the check: what it's called in the output, its arguments, and extra
/// environment variables
struct Step {
    /// Shown before it runs and when it fails
    name: &'static str,
    /// The cargo arguments, starting with the subcommand or a `+toolchain`
    args: Vec<String>,
    /// Set for this call only
    env: Vec<(&'static str, &'static str)>,
}

impl Step {
    /// A step without extra environment variables
    fn new(name: &'static str, args: &[&str]) -> Self {
        Step {
            name,
            args: args.iter().map(ToString::to_string).collect(),
            env: Vec::new(),
        }
    }
}

/// Runs the steps in order and stops at the first one that fails
fn check(fix: bool) -> ExitCode {
    let root = workspace_root();
    for step in steps(fix, &root) {
        println!("== {}", step.name);
        if !run(&step, &root) {
            eprintln!("== {} failed", step.name);
            return ExitCode::FAILURE;
        }
    }
    println!("== all checks passed");
    ExitCode::SUCCESS
}

/// The steps of `check`. The MSRV step is left out, with a note, when its toolchain isn't
/// installed
fn steps(fix: bool, root: &Path) -> Vec<Step> {
    let fmt = if fix {
        Step::new("fmt", &["fmt", "--all"])
    } else {
        Step::new("fmt", &["fmt", "--all", "--", "--check"])
    };
    let mut steps = vec![
        fmt,
        Step::new(
            "clippy",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        Step::new("test", &["test", "--workspace"]),
        Step {
            env: vec![("RUSTDOCFLAGS", "-D warnings")],
            ..Step::new(
                "doc",
                &["doc", "--workspace", "--no-deps", "--all-features"],
            )
        },
    ];
    match msrv(root) {
        Some(version) => match installed_toolchain(&version) {
            Some(toolchain) => steps.push(msrv_step(&toolchain)),
            None => println!(
                "note: the MSRV check is skipped, Rust {version} isn't installed \
             (rustup toolchain install {version})"
            ),
        },
        None => println!("note: the MSRV check is skipped, no rust-version in Cargo.toml"),
    }
    steps
}

/// Builds everything with the oldest supported Rust, through rustup's `+toolchain`
fn msrv_step(version: &str) -> Step {
    Step {
        name: "msrv",
        args: vec![
            format!("+{version}"),
            "check".into(),
            "--workspace".into(),
            "--all-targets".into(),
            "--locked".into(),
        ],
        env: Vec::new(),
    }
}

/// Runs one step with cargo in the workspace root; true if it succeeded
fn run(step: &Step, root: &Path) -> bool {
    // `+toolchain` only works through rustup's cargo proxy, not the toolchain's cargo itself
    let cargo = if step.args[0].starts_with('+') {
        "cargo".into()
    } else {
        cargo()
    };
    Command::new(cargo)
        .args(&step.args)
        .envs(step.env.iter().copied())
        .current_dir(root)
        .status()
        .is_ok_and(|status| status.success())
}

/// The cargo that runs this task, so the same toolchain checks the code
fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// The workspace root: the parent of this crate's directory
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is a directory in the workspace root")
        .to_path_buf()
}

/// `rust-version` from the root Cargo.toml, like `1.88`
fn msrv(root: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).ok()?;
    rust_version(&manifest)
}

/// The value of the first `rust-version = "…"` line
fn rust_version(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let value = line.trim().strip_prefix("rust-version")?.trim_start();
        let value = value.strip_prefix('=')?.trim();
        Some(value.trim_matches('"').to_string())
    })
}

/// The installed rustup toolchain for `version`: `1.88` is often installed as `1.88.0`. None
/// without rustup
fn installed_toolchain(version: &str) -> Option<String> {
    let patch = format!("{version}.0");
    let candidates = if version.matches('.').count() == 1 {
        vec![version, patch.as_str()]
    } else {
        vec![version]
    };
    candidates
        .into_iter()
        .find(|toolchain| toolchain_installed(toolchain))
        .map(ToString::to_string)
}

/// Whether rustup has the toolchain `version`; false without rustup
fn toolchain_installed(version: &str) -> bool {
    Command::new("rustup")
        .args(["run", version, "cargo", "--version"])
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rust_version_is_read_from_the_manifest() {
        let manifest = "[workspace.package]\nedition = \"2021\"\nrust-version = \"1.88\"\n";

        assert_eq!(rust_version(manifest).as_deref(), Some("1.88"));
    }

    #[test]
    fn a_manifest_without_rust_version_has_none() {
        assert_eq!(rust_version("[package]\nname = \"x\"\n"), None);
    }

    #[test]
    fn fix_formats_instead_of_checking() {
        let root = workspace_root();

        assert_eq!(steps(true, &root)[0].args, ["fmt", "--all"]);
        assert_eq!(
            steps(false, &root)[0].args,
            ["fmt", "--all", "--", "--check"]
        );
    }
}
