use std::collections::HashMap;

use anyhow::Result;
use hickory_resolver::config::ResolverConfig;
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::Resolver;
use reqwest::Url;
use tokio::runtime::Runtime;

pub fn txt_record_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let resolver = Resolver::builder_with_config(
        ResolverConfig::default(),
        TokioConnectionProvider::default(),
    )
    .build();

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse host"))?;

    let rt = Runtime::new()?;
    let records = rt.block_on(resolver.txt_lookup(host))?;

    for record in records {
        if let Some((key, value)) = record.to_string().split_once('=') {
            contexts.insert(key.to_string(), value.to_string());
        }
    }

    Ok(())
}
