//! `file+toml://` and `file+yaml://`: variables from local files

use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use toml::Value;

/// Reads the top-level keys of a TOML file
pub fn toml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = toml::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key, plain(value));
    }

    Ok(())
}

/// Reads the top-level keys of a YAML file
pub fn yaml_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let path = url.path();

    let contents = std::fs::read_to_string(path)?;
    let values: HashMap<String, Value> = serde_yaml_ng::from_str(&contents)?;

    for (key, value) in values {
        contexts.insert(key, plain(value));
    }

    Ok(())
}

/// The value as text: a string as it is, anything else as TOML, like `3` or `[1, 2]`
fn plain(value: Value) -> String {
    match value {
        Value::String(string) => string,
        value => value.to_string(),
    }
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    /// The variables of `contents`, written to a file and read with `read`
    fn variables(
        contents: &str,
        read: fn(&Url, &mut HashMap<String, String>) -> Result<()>,
    ) -> HashMap<String, String> {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vars");
        std::fs::write(&file, contents).unwrap();

        let mut variables = HashMap::new();
        read(&Url::from_file_path(&file).unwrap(), &mut variables).unwrap();
        variables
    }

    #[test]
    fn strings_have_no_quotes() {
        let yaml = variables("ship: Daedalus\ncrew: 3\n", yaml_values);
        let toml = variables("ship = \"Daedalus\"\ncrew = 3\n", toml_values);

        for variables in [yaml, toml] {
            assert_eq!("Daedalus", variables["ship"]);
            assert_eq!("3", variables["crew"]);
        }
    }
}
