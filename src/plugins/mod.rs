use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::contexts::{to_env_vars, to_koto, Contexts};
use anyhow::anyhow;
use ignore::WalkBuilder;
use koto::{Koto, KotoSettings};
use serde::{Deserialize, Deserializer};
use tracing::{debug, trace};
use which::which;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    Koto,
    Shell,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Plugin {
    pub name: String,
    pub description: String,
    pub runtime: PluginRuntime,

    #[serde(deserialize_with = "path_buf_from_string")]
    pub plan_script: PathBuf,

    #[serde(deserialize_with = "path_buf_from_string")]
    pub run_script: PathBuf,
}

pub enum PluginPhase {
    Plan,
    Run,
}

impl ToString for PluginPhase {
    fn to_string(&self) -> String {
        match *self {
            PluginPhase::Plan => "plan".to_string(),
            PluginPhase::Run => "run".to_string(),
        }
    }
}

fn path_buf_from_string<'de, D>(deserializer: D) -> anyhow::Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let path: String = Deserialize::deserialize(deserializer)?;
    Ok(PathBuf::from(path))
}

pub fn locate_plugins(plugins_dir: &Path) -> anyhow::Result<Vec<Plugin>> {
    let mut plugins = vec![];

    let mut walker = WalkBuilder::new(&plugins_dir);
    walker
        .standard_filters(true)
        .follow_links(false)
        .same_file_system(true)
        .max_depth(Some(2))
        .build()
        .filter(|entry| entry.clone().unwrap().path().ends_with("plugin.toml"))
        .for_each(|entry| {
            if let Ok(entry) = entry {
                let path = entry.path();

                if let Ok(plugin_toml) = read_to_string(path) {
                    match toml::from_str::<Plugin>(&plugin_toml) {
                        Ok(mut plugin) => {
                            let parent = entry.path().parent().unwrap(); // safe to unwrap

                            plugin.plan_script = absolute_path(parent, plugin.plan_script);
                            plugin.run_script = absolute_path(parent, plugin.run_script);

                            plugins.push(plugin);
                        }
                        Err(err) => println!("{}", err),
                    }
                }
            }
        });

    Ok(plugins)
}

fn absolute_path(base_path: &Path, path: PathBuf) -> PathBuf {
    if path.is_relative() {
        if let Some(lel) = path.file_name() {
            return base_path.join(lel);
        }
    }

    path
}

pub fn execute_plugin(
    plugin: &Plugin,
    contexts: &Contexts,
    phase: PluginPhase,
) -> anyhow::Result<bool> {
    let script_name = match phase {
        PluginPhase::Plan => &plugin.plan_script,
        PluginPhase::Run => &plugin.run_script,
    };

    let script = read_to_string(script_name)?;

    debug!("Plugin phase is '{}'", phase.to_string());

    match plugin.runtime {
        PluginRuntime::Koto => {
            let mut koto = Koto::with_settings(KotoSettings {
                run_tests: false,
                ..Default::default()
            });

            for (key, value) in contexts {
                koto.prelude().add_value(key, to_koto(value));
            }

            koto.prelude().add_value(
                "plugin.phase",
                koto::runtime::Value::Str(phase.to_string().into()),
            );

            let chunk = koto
                .compile(&script)
                .map_err(|err| anyhow!("{}", err.to_string()))?;

            let result = koto
                .run_chunk(chunk)
                .map_err(|err| anyhow!("{}", err.to_string()))?;

            match result {
                koto_runtime::Value::Bool(result) => Ok(result),
                _ => Err(anyhow!("Output is not a boolean")),
            }
        }
        PluginRuntime::Shell => {
            let shell = if cfg!(target_os = "windows") {
                "cmd".to_string()
            } else {
                // Detect scripts:
                // - ".sh" -> "sh" binary
                // - ".bash" -> "bash" binary
                let extension = script_name
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("sh");

                match which(extension) {
                    Ok(_) => extension.to_string(),
                    Err(_) => "sh".to_string(),
                }
            };

            debug!("Resolved shell is '{}'", shell);

            let mut env_vars = to_env_vars(contexts);
            env_vars.insert("PLUGIN_PHASE".to_string(), phase.to_string());

            let command = Command::new(shell)
                .arg(script_name)
                .env_clear()
                .envs(&env_vars)
                .stdin(Stdio::null())
                .spawn()?;

            let output = command.wait_with_output()?;

            let stdout = String::from_utf8(output.stdout)?;
            let stderr = String::from_utf8(output.stderr)?;

            trace!("exit code: {}", &output.status);
            trace!("stdout: {}", stdout);
            trace!("stderr: {}", stderr);

            Ok(output.status.success())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{execute_plugin, locate_plugins, Plugin};
    use fs_extra::file::write_all;
    use serde_yaml::Value;
    use std::{collections::BTreeMap, fs::create_dir, path::PathBuf};
    use tempfile::{tempdir, NamedTempFile};

    fn create_file_with_content(content: String) -> anyhow::Result<NamedTempFile> {
        let temp_file = NamedTempFile::new()?;
        write_all(&temp_file, &content)?;

        Ok(temp_file)
    }

    #[test]
    fn locate_plugins_works() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;

        let plugin_dir = temp_dir.path().join("test-plugin");
        create_dir(&plugin_dir)?;

        let plan_script_path = plugin_dir.join("plan.koto");
        write_all(plan_script_path, "true")?;

        let plugin_toml_path = plugin_dir.join("plugin.toml");
        write_all(
            plugin_toml_path,
            r#"name = "test-plugin"
description = "some description"
runtime = "koto"
plan_script = "plan.koto"
run_script = """#,
        )?;

        let plugins = locate_plugins(&temp_dir.path().to_path_buf())?;

        assert_eq!(plugins.len(), 1);

        Ok(())
    }

    #[test]
    fn plan_koto_plugin_fails_because_koto_script_returns_false() -> anyhow::Result<()> {
        let script_file = create_file_with_content("false".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: script_file.path().to_path_buf(),
            run_script: PathBuf::new(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Plan).unwrap(),
            false
        );

        Ok(())
    }

    #[test]
    fn plan_koto_plugin_fails_because_koto_script_does_not_compile() -> anyhow::Result<()> {
        let script_file = create_file_with_content("fasle".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: script_file.path().to_path_buf(),
            run_script: PathBuf::new(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Plan).is_ok(),
            false
        );

        Ok(())
    }

    #[test]
    fn plan_koto_plugin_succeeds() -> anyhow::Result<()> {
        let script_file = create_file_with_content("true".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: script_file.path().to_path_buf(),
            run_script: PathBuf::new(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Plan).unwrap(),
            true
        );

        Ok(())
    }

    #[test]
    fn run_koto_plugin_fails_because_koto_script_returns_false() -> anyhow::Result<()> {
        let script_file = create_file_with_content("false".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: PathBuf::new(),
            run_script: script_file.path().to_path_buf(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Run).unwrap(),
            false
        );

        Ok(())
    }

    #[test]
    fn run_koto_plugin_fails_because_koto_script_does_not_compile() -> anyhow::Result<()> {
        let script_file = create_file_with_content("fasle".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: PathBuf::new(),
            run_script: script_file.path().to_path_buf(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Run).is_ok(),
            false
        );

        Ok(())
    }

    #[test]
    fn run_koto_plugin_succeeds() -> anyhow::Result<()> {
        let script_file = create_file_with_content("some.value == \"fkbr\"".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Koto,
            plan_script: PathBuf::new(),
            run_script: script_file.path().to_path_buf(),
        };

        let mut contexts = BTreeMap::new();
        let mut values = BTreeMap::new();
        values.insert("value".to_string(), Value::String("fkbr".to_string()));

        contexts.insert("some".to_string(), values);

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Run).unwrap(),
            true
        );

        Ok(())
    }

    #[test]
    fn plan_shell_plugin_fails_because_shell_script_returns_false() -> anyhow::Result<()> {
        let script_file = create_file_with_content("exit 1".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Shell,
            plan_script: script_file.path().to_path_buf(),
            run_script: PathBuf::new(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Plan).unwrap(),
            false
        );

        Ok(())
    }

    #[test]
    fn plan_shell_plugin_succeeds() -> anyhow::Result<()> {
        let script_file = create_file_with_content("exit 0".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Shell,
            plan_script: script_file.path().to_path_buf(),
            run_script: PathBuf::new(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Plan).is_ok(),
            true
        );

        Ok(())
    }

    #[test]
    fn run_shell_plugin_fails_because_shell_script_returns_false() -> anyhow::Result<()> {
        let script_file = create_file_with_content("exit 1".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Shell,
            plan_script: PathBuf::new(),
            run_script: script_file.path().to_path_buf(),
        };

        let contexts = BTreeMap::new();

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Run).unwrap(),
            false
        );

        Ok(())
    }

    #[test]
    fn run_shell_plugin_succeeds() -> anyhow::Result<()> {
        let script_file =
            create_file_with_content("[ $SOME_VALUE = \"fkbr\" ] && exit 0".to_string())?;

        let plugin = Plugin {
            name: "demo.plugin".to_string(),
            description: "some description".to_string(),
            runtime: super::PluginRuntime::Shell,
            plan_script: PathBuf::new(),
            run_script: script_file.path().to_path_buf(),
        };

        let mut contexts = BTreeMap::new();
        let mut values = BTreeMap::new();
        values.insert("value".to_string(), Value::String("fkbr".to_string()));
        contexts.insert("some".to_string(), values);

        assert_eq!(
            execute_plugin(&plugin, &contexts, super::PluginPhase::Run).unwrap(),
            true
        );

        Ok(())
    }

    #[test]
    fn locate_and_run_plugins_work() -> anyhow::Result<()> {
        let temp_dir = tempdir()?;

        let plugin_dir = temp_dir.path().join("test-plugin");
        create_dir(&plugin_dir)?;

        let plan_script_path = plugin_dir.join("plan.koto");
        write_all(plan_script_path, "true")?;

        let plugin_toml_path = plugin_dir.join("plugin.toml");
        write_all(
            plugin_toml_path,
            r#"name = "test-plugin"
description = "some description"
runtime = "koto"
plan_script = "plan.koto"
run_script = """#,
        )?;

        let plugins = locate_plugins(&temp_dir.path().to_path_buf())?;

        assert_eq!(plugins.len(), 1);

        for plugin in plugins {
            let result =
                execute_plugin(&plugin, &BTreeMap::new(), crate::plugins::PluginPhase::Plan);

            assert_eq!(result.unwrap(), true);
        }

        Ok(())
    }
}
