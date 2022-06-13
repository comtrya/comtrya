use rhai::Scope;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::BTreeMap;
use tracing::{debug, instrument};
use user::UserContextProvider;

use crate::{
    config::Config,
    contexts::{
        env::EnvContextProvider, os::OSContextProvider, variables::VariablesContextProvider,
    },
};

pub mod env;
pub mod os;
/// User context provider: understands the user running the command
pub mod user;
pub mod variables;

pub trait ContextProvider {
    fn get_prefix(&self) -> String;
    fn get_contexts(&self) -> Vec<Context>;
}

pub type Contexts = BTreeMap<String, BTreeMap<String, Value>>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Context {
    KeyValueContext(String, String),
    ListContext(String, Vec<String>),
}

#[instrument(skip(config))]
pub fn build_contexts(config: &Config) -> Contexts {
    let mut contexts: Contexts = BTreeMap::new();

    let context_providers: Vec<Box<dyn ContextProvider>> = vec![
        Box::new(UserContextProvider {}),
        Box::new(OSContextProvider {}),
        Box::new(EnvContextProvider {}),
        Box::new(VariablesContextProvider { config }),
    ];

    context_providers.iter().for_each(|provider| {
        let mut values: BTreeMap<String, Value> = BTreeMap::new();

        provider
            .get_contexts()
            .iter()
            .for_each(|context| match context {
                Context::KeyValueContext(k, v) => {
                    debug!(
                        context = provider.get_prefix().as_str(),
                        key = k.clone().as_str(),
                        value = v.clone().as_str(),
                        message = ""
                    );
                    values.insert(k.clone(), v.clone().into());
                }
                Context::ListContext(k, v) => {
                    debug!(
                        context = provider.get_prefix().as_str(),
                        key = k.clone().as_str(),
                        values = v.clone().join(",").as_str(),
                        message = ""
                    );

                    values.insert(k.clone(), v.clone().into());
                }
            });

        contexts.insert(provider.get_prefix(), values);
    });

    contexts
}

pub fn to_tera(contexts: &Contexts) -> tera::Context {
    let mut context = tera::Context::new();

    contexts.iter().for_each(|(m, v)| context.insert(m, v));
    context
}

pub fn to_rhai(context: &Contexts) -> rhai::Scope {
    let mut scope = Scope::new();

    context.iter().for_each(|(m, v)| {
        let dynamic = match rhai::serde::to_dynamic(v) {
            Ok(dynamic) => dynamic,
            Err(error) => {
                panic!("Failed to convert context value to dynamic: {}", error);
            }
        };

        scope.push_constant(m.clone(), dynamic);
    });

    scope
}

#[cfg(test)]
mod test {
    use super::*;
    use rhai::Engine;
    use serde_yaml::Value;

    #[test]
    fn it_can_convert_to_rhai() {
        let engine = Engine::new();

        let mut contexts: Contexts = BTreeMap::new();
        let mut user_context: BTreeMap<String, Value> = BTreeMap::new();

        user_context.insert(String::from("username"), String::from("rawkode").into());
        contexts.insert(String::from("user"), user_context);

        let mut rhai_context = to_rhai(&contexts);
        assert!(rhai_context.contains("user"));

        let result = engine
            .eval_with_scope::<String>(&mut rhai_context, "user.username")
            .unwrap();
        assert_eq!(result, String::from("rawkode"));
    }

    #[test]
    fn variables_context_resolves_from_config() -> anyhow::Result<()> {
        let mut variables = BTreeMap::new();
        variables.insert("ship_name".to_string(), "Jack O'Neill".to_string());
        variables.insert("ship_captain".to_string(), "Thor".to_string());

        let config = Config {
            manifest_paths: vec![],
            variables,
            plugins_path: String::default(),
        };

        let contexts = build_contexts(&config);
        let variables_context_values = contexts.get("variables");

        assert_eq!(variables_context_values.is_some(), true);
        assert_eq!(
            variables_context_values.unwrap().get("ship_name").unwrap(),
            "Jack O'Neill"
        );
        assert_eq!(
            variables_context_values
                .unwrap()
                .get("ship_captain")
                .unwrap(),
            "Thor"
        );

        Ok(())
    }

    #[test]
    fn env_context() -> anyhow::Result<()> {
        let variables = BTreeMap::new();

        let config = Config {
            manifest_paths: vec![],
            variables,
            plugins_path: String::default(),
        };

        std::env::set_var("ASCENDED_NAME", "Morgan Le Fay");
        std::env::set_var("REAL_NAME", "Ganos Lal");

        let contexts = build_contexts(&config);
        let env_context_values = contexts.get("env");

        assert_eq!(env_context_values.is_some(), true);
        assert_eq!(
            env_context_values.unwrap().get("ASCENDED_NAME").unwrap(),
            "Morgan Le Fay"
        );
        assert_eq!(
            env_context_values.unwrap().get("REAL_NAME").unwrap(),
            "Ganos Lal"
        );

        Ok(())
    }
}
