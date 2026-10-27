//! The facts that become contexts

use super::{os::OsFacts, privilege::Privilege, user::UserFacts, variable_include, Contexts};
use crate::{actions::PackageProviders, config::Config, values::Value};
use std::collections::BTreeMap;
use tracing::{instrument, trace, warn};

/// Everything comtrya knows about the system and its configuration before it loads manifests.
/// Manifests see it as contexts, see [`Facts::to_contexts`]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facts {
    /// The user running comtrya
    pub user: UserFacts,
    /// The operating system
    pub os: OsFacts,
    /// The environment comtrya runs in
    pub env: BTreeMap<String, String>,
    /// `variables` in `Comtrya.yaml`
    pub variables: BTreeMap<String, String>,
    /// Read from the sources in `include_variables` in `Comtrya.yaml`
    pub include_variables: BTreeMap<String, String>,
    /// The program that runs privileged commands
    pub privilege: Privilege,
    /// `package_provider` in `Comtrya.yaml`, else the default for this OS
    pub package_provider: Option<PackageProviders>,
}

impl Facts {
    /// Looks at the system and the configuration. A source in `include_variables` that fails is
    /// logged, and its variables are left out
    #[instrument(skip(config))]
    pub fn gather(config: &Config) -> Self {
        let include_variables = variable_include::gather(config).unwrap_or_else(|err| {
            warn!("Error getting contexts from provider: include_variables -> {err}");
            BTreeMap::new()
        });

        Self {
            user: UserFacts::gather(),
            os: OsFacts::gather(),
            env: std::env::vars().collect(),
            variables: config.variables.clone(),
            include_variables,
            privilege: config.privilege.clone(),
            package_provider: config
                .package_provider
                .clone()
                .or_else(PackageProviders::for_os),
        }
    }

    /// The contexts that templates and `where` conditions see, like `user.home_dir`
    pub fn to_contexts(&self) -> Contexts {
        let strings = |values: &BTreeMap<String, String>| -> BTreeMap<String, Value> {
            values
                .iter()
                .map(|(key, value)| (key.clone(), value.clone().into()))
                .collect()
        };

        let contexts = Contexts::from([
            (String::from("user"), self.user.to_context()),
            (String::from("os"), self.os.to_context()),
            (String::from("env"), strings(&self.env)),
            (String::from("variables"), strings(&self.variables)),
            (
                String::from("include_variables"),
                strings(&self.include_variables),
            ),
            (
                String::from("privilege"),
                BTreeMap::from([(String::from("privilege"), self.privilege.to_string().into())]),
            ),
            (
                String::from("packages"),
                self.package_provider
                    .iter()
                    .map(|provider| (String::from("provider"), provider.as_str().into()))
                    .collect(),
            ),
        ]);

        // Only the keys: env and included variables often hold secrets
        for (prefix, values) in &contexts {
            trace!(context = prefix.as_str(), keys = ?values.keys().collect::<Vec<_>>());
        }

        contexts
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn keys(contexts: &Contexts, prefix: &str) -> Vec<String> {
        contexts[prefix].keys().cloned().collect()
    }

    /// Manifests use these names, so they must not change
    #[test]
    fn contexts_keep_their_names() {
        let facts = Facts {
            env: BTreeMap::from([(String::from("HOME"), String::from("/home/me"))]),
            variables: BTreeMap::from([(String::from("ship"), String::from("Daedalus"))]),
            include_variables: BTreeMap::from([(String::from("token"), String::from("…"))]),
            package_provider: Some(PackageProviders::Pacman),
            ..Default::default()
        };
        let contexts = facts.to_contexts();

        assert_eq!(
            vec![
                "env",
                "include_variables",
                "os",
                "packages",
                "privilege",
                "user",
                "variables"
            ],
            contexts.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            vec![
                "config_dir",
                "data_dir",
                "data_local_dir",
                "document_dir",
                "home_dir",
                "id",
                "name",
                "username"
            ],
            keys(&contexts, "user")
        );
        assert_eq!(
            vec![
                "bitness",
                "codename",
                "distribution",
                "edition",
                "family",
                "hostname",
                "name",
                "version"
            ],
            keys(&contexts, "os")
        );
        assert_eq!("/home/me", contexts["env"]["HOME"].to_string());
        assert_eq!("Daedalus", contexts["variables"]["ship"].to_string());
        assert_eq!("…", contexts["include_variables"]["token"].to_string());
        assert_eq!("sudo", contexts["privilege"]["privilege"].to_string());
        assert_eq!("pacman", contexts["packages"]["provider"].to_string());
    }

    #[test]
    fn without_a_package_provider_the_packages_context_is_empty() {
        let contexts = Facts::default().to_contexts();

        assert!(contexts["packages"].is_empty());
    }

    #[test]
    fn missing_user_dirs_are_unknown() {
        let contexts = Facts::default().to_contexts();

        assert_eq!("unknown", contexts["user"]["home_dir"].to_string());
        assert_eq!("0", contexts["user"]["id"].to_string());
    }

    #[test]
    fn a_broken_include_leaves_the_other_facts() {
        let facts = Facts::gather(&Config {
            include_variables: Some(vec![String::from("nope://what")]),
            variables: BTreeMap::from([(String::from("ship"), String::from("Daedalus"))]),
            ..Default::default()
        });

        assert!(facts.include_variables.is_empty());
        assert_eq!("Daedalus", facts.variables["ship"]);
    }
}
