use super::FileAction;
use super::{default_chmod, from_octal, OctalMode};
#[cfg(unix)]
use crate::atoms::file::Chown;
use crate::atoms::file::Decrypt;
use crate::manifests::Manifest;
use crate::steps::Step;
use crate::tera_functions::register_functions;
use crate::{actions::Action, contexts::to_tera};
use anyhow::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use std::path::PathBuf;
use tera::Tera;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCopy {
    #[serde(alias = "source")]
    pub from: String,

    #[serde(alias = "target")]
    pub to: String,

    #[serde(default = "default_chmod", deserialize_with = "from_octal")]
    #[schemars(with = "OctalMode")]
    pub chmod: u32,

    #[serde(default = "default_template")]
    pub template: bool,

    pub passphrase: Option<String>,

    /// A command (program and arguments) that gets the source file on stdin. Its stdout is
    /// written to `to` instead, e.g. `[gpg, --decrypt]`
    pub process_with: Option<Vec<String>>,

    #[serde(rename = "owned_by_user")]
    pub owner_user: Option<String>,

    #[serde(rename = "owned_by_group")]
    pub owner_group: Option<String>,
}

fn default_template() -> bool {
    false
}

impl FileCopy {}

impl FileAction for FileCopy {}

impl Action for FileCopy {
    fn summarize(&self) -> String {
        format!("Copy file from {} to {}", self.from, self.to)
    }

    fn plan(
        &self,
        manifest: &Manifest,
        context: &crate::contexts::Contexts,
    ) -> anyhow::Result<Vec<Step>> {
        if self.passphrase.is_some() && self.process_with.is_some() {
            return Err(anyhow!(
                "FileCopy can use either passphrase or process_with, not both"
            ));
        }

        let contents = match self.load(manifest, &self.from) {
            Ok(contents) => {
                let contents = match &self.process_with {
                    Some(command) => process(command, contents)?,
                    None => contents,
                };

                if self.template {
                    let mut tera = Tera::default();
                    register_functions(&mut tera);

                    let content_as_str = std::str::from_utf8(&contents)?;

                    match tera.render_str(content_as_str, &to_tera(context)) {
                        Ok(rendered) => rendered,
                        Err(err) => match err.source() {
                            Some(source) => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {source}"
                                ));
                            }
                            None => {
                                return Err(anyhow!(
                                    "Failed to render contents for FileCopy action: {err}"
                                ));
                            }
                        },
                    }
                    .as_bytes()
                    .to_vec()
                } else {
                    contents
                }
            }
            Err(err) => {
                return Err(anyhow!("Failed to get contents for FileCopy action: {err}"));
            }
        };

        use crate::atoms::directory::Create as DirCreate;
        use crate::atoms::file::{Chmod, Create, SetContents};

        let mut path = PathBuf::from(&self.to);

        if path.is_dir() {
            if let Some(file_name) = PathBuf::from(self.from.clone()).file_name() {
                path = path.join(file_name);
            }
        }

        let parent = path.clone();
        let mut steps = vec![
            Step::new(DirCreate {
                path: parent
                    .parent()
                    .ok_or_else(|| anyhow!("Failed to get parent directory for FileCopy action"))?
                    .into(),
            }),
            Step::new(Create { path: path.clone() }),
            Step::new(Chmod {
                path: path.clone(),
                mode: self.chmod,
            }),
        ];

        #[cfg(unix)]
        let path_for_chown = path.clone();
        if let Some(passphrase) = self.passphrase.to_owned() {
            steps.push(Step::new(Decrypt {
                encrypted_content: contents,
                path,
                passphrase,
            }));
        } else {
            steps.push(Step::new(SetContents {
                path,
                contents,
                redact: self.process_with.is_some(),
            }));
        }

        #[cfg(unix)]
        if let Some(user) = self.owner_user.clone() {
            if let Some(group) = self.owner_group.clone() {
                steps.push(Step::new(Chown {
                    path: path_for_chown,
                    owner: user.clone(),
                    group: group.clone(),
                }))
            }
        }

        Ok(steps)
    }
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
            .plan(&Default::default(), &Default::default())
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
}
