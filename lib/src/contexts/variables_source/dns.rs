use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;
use trust_dns_resolver::config::ResolverConfig;
use trust_dns_resolver::config::ResolverOpts;
use trust_dns_resolver::Resolver;

pub fn txt_record_values(url: &Url, contexts: &mut HashMap<String, String>) -> Result<()> {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default())?;

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse host"))?;

    let records = resolver.txt_lookup(host)?;

    for record in records {
        if let Some((key, value)) = record.to_string().split_once('=') {
            contexts.insert(key.to_string(), value.to_string());
        }
    }

    Ok(())
}
