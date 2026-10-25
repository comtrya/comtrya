use crate::actions::PackageProviders;
use crate::contexts::privilege::Privilege;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub manifest_paths: Vec<String>,

    #[serde(default)]
    pub variables: BTreeMap<String, String>,

    #[serde(default)]
    pub include_variables: Option<Vec<String>>,

    #[serde(default)]
    pub disable_update_check: bool,

    #[serde(default)]
    pub privilege: Privilege,

    /// Used by `package.*` actions without a `provider`, instead of the default for the OS
    #[serde(default)]
    pub package_provider: Option<PackageProviders>,
}
