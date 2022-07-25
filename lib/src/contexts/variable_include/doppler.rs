use anyhow::{anyhow, Result};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str;
use std::{collections::HashMap, process::Command};
use tracing::{trace, warn};
use which::which;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Secret {
    pub computed: String,
}

fn am_i_logged_in(cli: &PathBuf) -> Result<bool> {
    let output = Command::new(cli).arg("configure").arg("--json").output()?;

    if output.status.success() {
        let json = serde_json::from_slice::<serde_json::Value>(&output.stdout)?;

        let maybe_token = json.get("/").and_then(|value| value.get("token"));

        Ok(maybe_token.is_some())
    } else {
        warn!(
            "doppler call exited with status '{}' and output='{:?}'",
            output.status.code().unwrap_or_default(),
            output.stdout
        );

        Ok(false)
    }
}

pub fn secrets(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let cli = which("doppler")?;

    if !am_i_logged_in(&cli)? {
        return Err(anyhow!("You're not logged in!"));
    }

    let project = url
        .host()
        .ok_or_else(|| anyhow!("Cannot extract project"))?
        .to_string();
    let config_and_secret = url
        .path_segments()
        .map(|p| p.collect::<Vec<&str>>())
        .ok_or_else(|| anyhow!("Cannot extract config and secret"))?;

    let mut args = vec![];

    if config_and_secret.len() == 1 {
        let config = config_and_secret
            .get(0)
            .ok_or_else(|| anyhow!("Cannot extract config"))?;

        args.push("secrets");
        args.push("--project");
        args.push(&project);
        args.push("--config");
        args.push(config);
        args.push("--json");

        trace!(
            "Getting secrets for project='{}' and config='{}'",
            project,
            config
        );
    } else if config_and_secret.len() == 2 {
        let config = config_and_secret
            .get(0)
            .ok_or_else(|| anyhow!("Cannot extract config"))?;

        let secret = config_and_secret
            .get(1)
            .ok_or_else(|| anyhow!("Cannot extract secret"))?;

        args.push("secrets");
        args.push("get");
        args.push("--project");
        args.push(&project);
        args.push("--config");
        args.push(config);
        args.push(secret);
        args.push("--json");

        trace!(
            "Getting secret='{}' for project='{}' and config='{}'",
            secret,
            project,
            config
        );
    }

    if args.is_empty() {
        warn!("URL '{}' does not fit the schema", &url);
    }

    let output = Command::new(cli).args(args).output()?;

    if output.status.success() {
        let secrets = serde_json::from_slice::<BTreeMap<String, Secret>>(&output.stdout)?;

        for (key, value) in secrets {
            if !key.starts_with("DOPPLER_") {
                contexts.insert(key, value.computed);
            }
        }
    } else {
        warn!(
            "doppler call exited with status '{}' and output='{:?}'",
            output.status.code().unwrap_or_default(),
            output.stdout
        );
    }

    Ok(())
}
