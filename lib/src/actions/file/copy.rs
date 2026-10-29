//! `file.copy`: writes a file from the manifest's `files` directory, optionally rendered,
//! decrypted or piped through a command

use super::{change_owner, create_parent, default_chmod, from_octal, FileAction, OctalMode};
use crate::actions::{Action, PlanCx};
use crate::atoms::file::{Chmod, Create, Decrypt, SetContents};
use crate::contexts::{to_tera, Contexts};
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::tera_functions::register_functions;
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use tera::Tera;

/// Copies a file from the manifest's `files` directory, when its contents differ. Always with
/// `passphrase`
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCopy {
    /// The file to copy, below the manifest's `files` directory
    #[serde(alias = "source")]
    #[schemars(extend("x-aliases" = ["source"]))]
    pub from: String,

    /// Where to copy it to. Into a directory, the file keeps its name
    #[serde(alias = "target")]
    #[schemars(extend("x-aliases" = ["target"]))]
    pub to: String,

    /// The permissions of the file, `644` by default
    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    #[schemars(with = "OctalMode")]
    pub chmod: u32,

    /// Render the file as a Tera template with the contexts first
    #[serde(default = "default_template")]
    pub template: bool,

    /// Decrypt the file with this age passphrase. Can't be used with `process_with`
    pub passphrase: Option<String>,

    /// A command (program and arguments) that gets the source file on stdin. Its stdout is
    /// written to `to` instead, e.g. `[gpg, --decrypt]`
    pub process_with: Option<Vec<String>>,

    /// The owner of the file, on unix. Needs `owned_by_group` too
    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,

    /// The group of the file, on unix. Needs `owned_by_user` too
    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,
}

/// Files aren't templates unless `template` says so
fn default_template() -> bool {
    false
}

impl FileCopy {
    /// The source file, piped through `process_with` and rendered as a template if asked for
    fn contents(&self, manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<u8>> {
        let contents = self
            .load(manifest, &self.from)
            .map_err(|err| anyhow!("Failed to get contents for FileCopy action: {err}"))?;

        let contents = match &self.process_with {
            Some(command) => process(command, contents)?,
            None => contents,
        };

        if self.template {
            render(&contents, contexts)
        } else {
            Ok(contents)
        }
    }

    /// The file to write. When `to` is a directory, the file keeps its name inside of it
    fn target(&self) -> PathBuf {
        let path = PathBuf::from(&self.to);

        match Path::new(&self.from).file_name() {
            Some(file_name) if path.is_dir() => path.join(file_name),
            _ => path,
        }
    }

    /// Decrypts the contents with the passphrase, or writes them as they are
    fn write(&self, path: PathBuf, contents: Vec<u8>) -> Step {
        match &self.passphrase {
            Some(passphrase) => Step::new(Decrypt {
                encrypted_content: contents,
                path,
                passphrase: passphrase.clone(),
            }),
            None => Step::new(SetContents {
                path,
                contents,
                // The output of `process_with` is often a secret, so it's kept out of the diff
                redact: self.process_with.is_some(),
            }),
        }
    }
}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn summarize(&self) -> String {
        format!("Copy file from {} to {}", self.from, self.to)
    }

    fn plan(&self, manifest: &Manifest, cx: &PlanCx) -> anyhow::Result<Vec<Step>> {
        if self.passphrase.is_some() && self.process_with.is_some() {
            return Err(anyhow!(
                "FileCopy can use either passphrase or process_with, not both"
            ));
        }

        let contents = self.contents(manifest, cx.contexts)?;
        let path = self.target();

        let mut steps = vec![
            create_parent(&path)?,
            Step::new(Create { path: path.clone() }),
            Step::new(Chmod {
                path: path.clone(),
                mode: self.chmod,
            }),
            self.write(path.clone(), contents),
        ];
        steps.extend(change_owner(&path, &self.owner_user, &self.owner_group));

        Ok(steps)
    }
}

/// Renders `contents` as a Tera template with the contexts
fn render(contents: &[u8], contexts: &Contexts) -> anyhow::Result<Vec<u8>> {
    let mut tera = Tera::default();
    register_functions(&mut tera);

    let rendered = tera
        .render_str(std::str::from_utf8(contents)?, &to_tera(contexts))
        .map_err(|err| {
            // Tera's own message only says that rendering failed, the source says why
            let reason = err
                .source()
                .map_or(err.to_string(), |source| source.to_string());
            anyhow!("Failed to render contents for FileCopy action: {reason}")
        })?;

    Ok(rendered.into_bytes())
}

/// Runs `command` with `input` on stdin and returns its stdout
fn process(command: &[String], input: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let (program, args) = command
        .split_first()
        .ok_or_else(|| anyhow!("process_with needs at least a program to run"))?;

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| anyhow!("process_with could not run `{program}`: {err}"))?;

    // Write from a separate thread, so a command that writes before it has read
    // all of its input can't block on a full pipe
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let writer = std::thread::spawn(move || stdin.write_all(&input));

    let output = child.wait_with_output()?;
    // A command may exit without reading all of its input, which is fine
    let _ = writer.join();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "process_with `{}` failed with {}: {}",
            command.join(" "),
            output.status,
            stderr.trim()
        ));
    }

    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialized() {
        let yaml = r#"
- action: file.copy
  from: a
  to: b
  chmod: "0777"
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!(0o777, action.action.chmod);
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_be_deserialized_chown() {
        let yaml = r#"
- action: file.copy
  from: a
  to: b
  chmod: "0777"
  owned_by_user: test
  owned_by_group: test
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => {
                assert_eq!("a", action.action.from);
                assert_eq!("b", action.action.to);
                assert_eq!(0o777, action.action.chmod);
                assert_eq!("test", action.action.owner_user.as_deref().unwrap());
                assert_eq!("test", action.action.owner_group.as_deref().unwrap());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    fn chmod_of(yaml_value: &str) -> Result<u32, serde_yaml_ng::Error> {
        let yaml = format!("- action: file.copy\n  from: a\n  to: b\n  chmod: {yaml_value}\n");
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(&yaml)?;

        match actions.pop() {
            Some(Actions::FileCopy(action)) => Ok(action.action.chmod),
            _ => panic!("FileCopy didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn chmod_accepts_numbers_and_strings_as_octal() {
        // As documented: a plain number is read as octal digits
        assert_eq!(0o644, chmod_of("644").unwrap());
        assert_eq!(0o755, chmod_of("755").unwrap());
        assert_eq!(0o644, chmod_of("0644").unwrap());
        assert_eq!(0o600, chmod_of("\"0600\"").unwrap());
        assert_eq!(0o600, chmod_of("\"600\"").unwrap());
    }

    #[test]
    fn chmod_defaults_to_644() {
        let mut actions: Vec<Actions> =
            serde_yaml_ng::from_str("- action: file.copy\n  from: a\n  to: b\n").unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => assert_eq!(0o644, action.action.chmod),
            _ => panic!("FileCopy didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn chmod_rejects_non_octal_digits() {
        assert!(chmod_of("\"0999\"").is_err());
        assert!(chmod_of("rwx").is_err());
    }

    #[test]
    fn process_with_can_be_deserialized() {
        let yaml =
            "- action: file.copy\n  from: a.asc\n  to: b\n  process_with: [gpg, --decrypt]\n";
        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::FileCopy(action)) => assert_eq!(
                Some(vec![String::from("gpg"), String::from("--decrypt")]),
                action.action.process_with
            ),
            _ => panic!("FileCopy didn't deserialize to the correct type"),
        }
    }

    #[test]
    fn process_with_and_passphrase_are_exclusive() {
        use crate::actions::Action;

        let copy = super::FileCopy {
            from: String::from("a"),
            to: String::from("b"),
            passphrase: Some(String::from("secret")),
            process_with: Some(vec![String::from("cat")]),
            ..Default::default()
        };

        let err = copy
            .plan(&Default::default(), &crate::actions::PlanCx::empty())
            .err()
            .expect("plan must fail");
        assert!(err.to_string().contains("not both"), "{err}");
    }

    #[test]
    fn process_with_needs_a_program() {
        assert!(super::process(&[], vec![]).is_err());
    }

    #[test]
    fn process_with_reports_a_missing_program() {
        let err = super::process(&[String::from("comtrya-does-not-exist")], vec![]).unwrap_err();
        assert!(err.to_string().contains("comtrya-does-not-exist"), "{err}");
    }

    #[cfg(unix)]
    #[test]
    fn process_with_pipes_contents_through_the_command() {
        let command = [String::from("tr"), String::from("a-z"), String::from("A-Z")];
        let output = super::process(&command, b"hello".to_vec()).unwrap();
        assert_eq!(b"HELLO".to_vec(), output);
    }

    #[cfg(unix)]
    #[test]
    fn process_with_handles_output_larger_than_a_pipe_buffer() {
        let input = vec![b'x'; 1024 * 1024];
        let output = super::process(&[String::from("cat")], input.clone()).unwrap();
        assert_eq!(input, output);
    }

    #[cfg(unix)]
    #[test]
    fn process_with_reports_stderr_when_the_command_fails() {
        let command = [
            String::from("sh"),
            String::from("-c"),
            String::from("echo bad key >&2; exit 2"),
        ];
        let err = super::process(&command, vec![]).unwrap_err().to_string();
        assert!(err.contains("bad key"), "{err}");
    }

    fn copy(from: &str, to: &str) -> super::FileCopy {
        super::FileCopy {
            from: String::from(from),
            to: String::from(to),
            ..Default::default()
        }
    }

    #[test]
    fn the_file_keeps_its_name_when_copied_into_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let to = dir.path().to_str().unwrap();

        assert_eq!(dir.path().join("b.txt"), copy("a/b.txt", to).target());
        assert_eq!(
            dir.path().join("c.txt"),
            copy("a/b.txt", dir.path().join("c.txt").to_str().unwrap()).target()
        );
    }

    #[test]
    fn templates_are_rendered_with_the_contexts() {
        let mut contexts = crate::contexts::Contexts::default();
        contexts.insert(
            String::from("user"),
            [(String::from("name"), "ferris".into())]
                .into_iter()
                .collect(),
        );

        let rendered = super::render(b"hi {{ user.name }}", &contexts).unwrap();
        assert_eq!(b"hi ferris".to_vec(), rendered);

        let err = super::render(b"{{ nope }}", &contexts).unwrap_err();
        assert!(
            err.to_string().contains("Failed to render contents"),
            "{err}"
        );
    }

    #[test]
    fn plan_writes_the_file_and_sets_the_owner() {
        use crate::actions::Action;

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("files")).unwrap();
        std::fs::write(dir.path().join("files/a"), "hello").unwrap();
        let manifest = crate::manifests::Manifest {
            root_dir: Some(dir.path().to_path_buf()),
            ..Default::default()
        };
        let mut action = copy("a", dir.path().join("out/a").to_str().unwrap());
        action.owner_user = Some(String::from("ferris"));
        action.owner_group = Some(String::from("crabs"));

        let steps = action
            .plan(&manifest, &crate::actions::PlanCx::empty())
            .unwrap();

        // create the directory and the file, chmod, write, and chown on unix only
        assert_eq!(if cfg!(unix) { 5 } else { 4 }, steps.len());
    }
}
