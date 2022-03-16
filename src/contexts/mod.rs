use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::BTreeMap;
use tracing::{debug, trace};
use user::UserContextProvider;

use crate::{
    config::Config,
    contexts::{os::OSContextProvider, variables::VariablesContextProvider},
};

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

pub fn build_contexts(config: &Config) -> Contexts {
    trace!("Building Contexts");

    let mut contexts: Contexts = BTreeMap::new();

    let context_providers: Vec<Box<dyn ContextProvider>> = vec![
        Box::new(UserContextProvider {}),
        Box::new(OSContextProvider {}),
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

pub fn to_koto(context: &BTreeMap<String, Value>) -> koto::runtime::Value {
    koto_yaml::yaml_value_to_koto_value(&serde_yaml::to_value(context).unwrap()).unwrap()
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_yaml::Value;

    #[test]
    fn it_can_convert_to_tera() {
        let mut contexts: Contexts = BTreeMap::new();
        let mut user_context: BTreeMap<String, Value> = BTreeMap::new();

        user_context.insert(String::from("username"), String::from("rawkode").into());
        contexts.insert(String::from("user"), user_context);

        let tera_context = to_tera(&contexts);
        assert_eq!(true, tera_context.contains_key("user"));
        let tera_user_context = tera_context.get("user").unwrap();
        assert_eq!(true, tera_user_context.get("username").is_some());
        assert_eq!(
            String::from("rawkode"),
            tera_user_context.get("username").unwrap().as_str().unwrap()
        );
    }

    #[test]
    fn variables_context_resolves_from_config() -> anyhow::Result<()> {
        let mut variables = BTreeMap::new();
        variables.insert("ship_name".to_string(), "Jack O'Neill".to_string());
        variables.insert("ship_captain".to_string(), "Thor".to_string());

        let config = Config {
            manifests: vec![],
            variables: Some(variables),
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
}
