use rhai::{Engine, Scope};
use std::collections::BTreeMap;
use std::sync::LazyLock;
use tracing::trace;

use crate::{config::Config, values::Value};

pub mod facts;
pub mod os;
pub mod privilege;
/// The user running comtrya
pub mod user;
pub mod variable_include;

pub use facts::Facts;

pub type Contexts = BTreeMap<String, BTreeMap<String, Value>>;

/// Gathers the [`Facts`] and returns them as contexts
pub fn build_contexts(config: &Config) -> Contexts {
    Facts::gather(config).to_contexts()
}

pub fn to_tera(contexts: &Contexts) -> tera::Context {
    let mut context = tera::Context::new();

    contexts.iter().for_each(|(m, v)| context.insert(m, v));
    context
}

/// One engine for every `where` condition. `sync` makes it shareable between threads
static ENGINE: LazyLock<Engine> = LazyLock::new(|| {
    let mut engine = Engine::new();
    // A condition that loops forever stops with an error instead of hanging the run
    engine.set_max_operations(100_000);
    engine
});

/// Evaluates a `where` condition against the contexts
pub fn evaluate_condition(condition: &str, contexts: &Contexts) -> anyhow::Result<bool> {
    ENGINE
        .eval_with_scope::<bool>(&mut to_rhai(contexts)?, condition)
        .map_err(|err| anyhow::anyhow!("{err}"))
}

pub fn to_rhai(context: &Contexts) -> anyhow::Result<rhai::Scope<'_>> {
    let mut scope = Scope::new();

    for (m, v) in context {
        let dynamic = rhai::serde::to_dynamic(v)
            .map_err(|error| anyhow::anyhow!("context {m} can't be used in conditions: {error}"))?;

        // Only the name: env and included variables often hold secrets
        trace!("Add dynamic constant '{}'", &m);

        scope.push_constant(m.clone(), dynamic);
    }

    Ok(scope)
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;
    use rhai::Engine;

    #[test]
    fn conditions_are_evaluated_against_the_contexts() {
        let mut contexts: Contexts = BTreeMap::new();
        let mut user_context: BTreeMap<String, Value> = BTreeMap::new();
        user_context.insert(String::from("username"), String::from("rawkode").into());
        contexts.insert(String::from("user"), user_context);

        assert!(evaluate_condition(r#"user.username == "rawkode""#, &contexts).unwrap());
        assert!(!evaluate_condition(r#"user.username == "root""#, &contexts).unwrap());
        assert!(evaluate_condition("user.username ==", &contexts).is_err());
        assert!(evaluate_condition(r#"user.username"#, &contexts).is_err());
    }

    #[test]
    fn a_condition_that_never_ends_fails() {
        let error = evaluate_condition("loop {}", &BTreeMap::new()).unwrap_err();
        assert!(error.to_string().contains("Too many operations"), "{error}");
    }

    #[test]
    fn it_can_convert_to_rhai() {
        let engine = Engine::new();

        let mut contexts: Contexts = BTreeMap::new();
        let mut user_context: BTreeMap<String, Value> = BTreeMap::new();

        user_context.insert(String::from("username"), String::from("rawkode").into());
        contexts.insert(String::from("user"), user_context);

        let mut rhai_context = to_rhai(&contexts).unwrap();
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
            variables,
            ..Default::default()
        };

        let contexts = build_contexts(&config);
        let variables_context_values = contexts.get("variables");

        assert_eq!(variables_context_values.is_some(), true);
        assert_eq!(
            variables_context_values
                .unwrap()
                .get("ship_name")
                .unwrap()
                .to_string(),
            "Jack O'Neill".to_string()
        );
        assert_eq!(
            variables_context_values
                .unwrap()
                .get("ship_captain")
                .unwrap()
                .to_string(),
            "Thor".to_string()
        );

        Ok(())
    }

    #[test]
    fn env_context() -> anyhow::Result<()> {
        let variables = BTreeMap::new();

        let config = Config {
            variables,
            ..Default::default()
        };

        std::env::set_var("ASCENDED_NAME", "Morgan Le Fay");
        std::env::set_var("REAL_NAME", "Ganos Lal");

        let contexts = build_contexts(&config);
        let env_context_values = contexts.get("env");

        assert_eq!(env_context_values.is_some(), true);
        assert_eq!(
            env_context_values
                .unwrap()
                .get("ASCENDED_NAME")
                .unwrap()
                .to_string(),
            "Morgan Le Fay".to_string()
        );
        assert_eq!(
            env_context_values
                .unwrap()
                .get("REAL_NAME")
                .unwrap()
                .to_string(),
            "Ganos Lal".to_string()
        );

        Ok(())
    }
}
