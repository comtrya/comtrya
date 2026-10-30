//! `dns+txt://<host>`: variables from DNS TXT records

use std::collections::HashMap;

use anyhow::Result;
use hickory_resolver::config::{ResolverConfig, GOOGLE};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::proto::rr::RData;
use hickory_resolver::Resolver;
use reqwest::Url;

use crate::runtime;

/// Reads the TXT records of the host as `key=value`, using Google's DNS. Records without `=`
/// are left out
pub fn txt_record_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let resolver = Resolver::builder_with_config(
        ResolverConfig::udp_and_tcp(&GOOGLE),
        TokioRuntimeProvider::default(),
    )
    .build()?;

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse host"))?;

    let records = runtime::block_on(resolver.txt_lookup(host))?;

    for record in records.answers() {
        let RData::TXT(txt) = &record.data else {
            continue;
        };

        if let Some((key, value)) = txt.to_string().split_once('=') {
            contexts.insert(key.to_string(), value.to_string());
        }
    }

    Ok(())
}
