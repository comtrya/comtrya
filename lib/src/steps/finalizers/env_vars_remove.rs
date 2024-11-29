use crate::atoms::Atom;
use crate::steps::finalizers::Finalizer;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RemoveEnvVars(pub HashMap<String, String>);

impl Finalizer for RemoveEnvVars {
    fn finalize(&self, _atom: &dyn Atom) -> anyhow::Result<bool> {
        for (key, _value) in self.0.iter() {
            std::env::remove_var(key);
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::Echo;
    use std::env;

    #[test]
    fn test_env_vars() {
        let atom = Echo("goodbye-world");
        env::set_var("FOO", "bar");

        let map = HashMap::from([("FOO".to_string(), "bar".to_string())]);
        let finalizer = RemoveEnvVars(map);
        let result = finalizer.finalize(&atom);

        pretty_assertions::assert_eq!(true, result.is_ok());
        pretty_assertions::assert_eq!(true, result.unwrap());
        pretty_assertions::assert_eq!(true, std::env::var("FOO").is_err());
    }
}
