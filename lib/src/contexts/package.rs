use crate::actions::PackageProviders;
use crate::config::Config;
use crate::contexts::{Context, ContextProvider};

/// `packages.provider`: the provider `package.*` actions use when they don't set one
pub struct PackageContextProvider<'a> {
    pub config: &'a Config,
}

impl<'a> ContextProvider for PackageContextProvider<'a> {
    fn get_prefix(&self) -> String {
        "packages".to_string()
    }

    fn get_contexts(&self) -> anyhow::Result<Vec<Context>> {
        let provider = self
            .config
            .package_provider
            .clone()
            .or_else(PackageProviders::for_os);

        Ok(provider
            .map(|provider| {
                Context::KeyValueContext("provider".to_string(), provider.as_str().into())
            })
            .into_iter()
            .collect())
    }
}
