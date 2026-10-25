use std::{collections::HashMap, io::ErrorKind, process::Command};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;

/// The part of `op item get --format json` that holds the values
#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    fields: Vec<Field>,
}

#[derive(Deserialize)]
struct Field {
    #[serde(default)]
    label: String,
    #[serde(default)]
    value: Option<String>,
}

/// Loads the fields of a 1Password item, `op://<vault>/<item>[?account=<account>]`, with the
/// 1Password CLI. The values are secrets: they must never end up in an error or log message.
pub fn item_values(include: &str, contexts: &mut HashMap<String, String>) -> Result<()> {
    let (vault, item, account) = parse(include)?;

    let mut command = Command::new("op");
    command.args(["item", "get", item, "--vault", vault, "--format", "json"]);
    if let Some(account) = account {
        command.args(["--account", account]);
    }

    let output = command.output().map_err(|err| match err.kind() {
        ErrorKind::NotFound => anyhow!(
            "{include} needs the 1Password CLI `op`, see https://developer.1password.com/docs/cli/get-started/"
        ),
        _ => anyhow!("failed to run `op` for {include}: {err}"),
    })?;

    if !output.status.success() {
        bail!(
            "`op` failed to read {include}: {}. Sign in with `op signin`, or turn on the 1Password app integration",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    // serde_json errors can quote the input, so don't pass them on
    let item = serde_json::from_slice::<Item>(&output.stdout)
        .ok()
        .with_context(|| format!("`op` returned an item for {include} that can't be read"))?;

    contexts.extend(values(item));

    Ok(())
}

fn parse(include: &str) -> Result<(&str, &str, Option<&str>)> {
    let invalid =
        || anyhow!("{include} is not of the form op://<vault>/<item>[?account=<account>]");

    let rest = include.strip_prefix("op://").ok_or_else(invalid)?;
    let (path, query) = match rest.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (rest, None),
    };

    let (vault, item) = path.split_once('/').ok_or_else(invalid)?;
    if vault.is_empty() || item.is_empty() || item.contains('/') {
        return Err(invalid());
    }

    let account = match query {
        None => None,
        Some(query) => Some(
            query
                .strip_prefix("account=")
                .filter(|account| !account.is_empty() && !account.contains('&'))
                .ok_or_else(invalid)?,
        ),
    };

    Ok((vault, item, account))
}

/// Fields without a label or value, like an empty notes field, are left out
fn values(item: Item) -> impl Iterator<Item = (String, String)> {
    item.fields
        .into_iter()
        .filter_map(|field| match field.value {
            Some(value) if !field.label.is_empty() && !value.is_empty() => {
                Some((field.label, value))
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_vault_item_and_account() {
        assert_eq!(
            parse("op://Private/GitHub").unwrap(),
            ("Private", "GitHub", None)
        );
        assert_eq!(
            parse("op://My Vault/Work Laptop?account=my.1password.com").unwrap(),
            ("My Vault", "Work Laptop", Some("my.1password.com"))
        );
    }

    #[test]
    fn rejects_incomplete_references() {
        for include in [
            "op://Private",
            "op:///GitHub",
            "op://Private/",
            "op://Private/GitHub/password",
            "op://Private/GitHub?vault=x",
            "op://Private/GitHub?account=",
        ] {
            assert!(parse(include).is_err(), "{include}");
        }
    }

    #[test]
    fn reads_labelled_fields_with_values() {
        let json = r#"{
            "id": "abc",
            "title": "GitHub",
            "fields": [
                {"id": "username", "type": "STRING", "purpose": "USERNAME", "label": "username", "value": "octocat"},
                {"id": "password", "type": "CONCEALED", "purpose": "PASSWORD", "label": "password", "value": "hunter2"},
                {"id": "notesPlain", "type": "STRING", "purpose": "NOTES", "label": "notesPlain"},
                {"id": "x", "type": "STRING", "label": "empty", "value": ""},
                {"id": "y", "type": "STRING", "label": "", "value": "no label"}
            ]
        }"#;

        let item = serde_json::from_str::<Item>(json).unwrap();
        let mut values = values(item).collect::<Vec<_>>();
        values.sort();

        assert_eq!(
            values,
            vec![
                ("password".to_string(), "hunter2".to_string()),
                ("username".to_string(), "octocat".to_string()),
            ]
        );
    }
}
