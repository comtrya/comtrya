use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitClone {
    /// `repository` is the name comtrya 0.8.7 and earlier used
    #[serde(alias = "repository")]
    pub repo_url: String,

    pub directory: String,

    /// Branch or tag to check out instead of the remote's default branch
    #[serde(default)]
    pub reference: Option<String>,
}

impl Action for GitClone {
    fn summarize(&self) -> String {
        format!("Cloning repository {} to {}", self.repo_url, self.directory)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        let url = gix::url::parse(self.repo_url.as_str())?;
        Ok(vec![Step {
            atom: Box::new(crate::atoms::git::Clone {
                repository: url.clone(),
                directory: PathBuf::from(self.directory.clone()),
                reference: self.reference.clone(),
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;
    use pretty_assertions::assert_eq;

    fn parse(yaml: &str) -> super::GitClone {
        match serde_yaml_ng::from_str::<Vec<Actions>>(yaml).unwrap().pop() {
            Some(Actions::GitClone(action)) => action.action,
            _ => panic!("git.clone didn't deserialize to GitClone"),
        }
    }

    #[test]
    fn accepts_the_current_form() {
        let action = parse(
            "- action: git.clone\n  repo_url: https://github.com/comtrya/comtrya\n  directory: /tmp/comtrya\n",
        );

        assert_eq!("https://github.com/comtrya/comtrya", action.repo_url);
        assert_eq!(None, action.reference);
    }

    /// The form of 0.8.7 and earlier: `repository` and an optional `reference`
    #[test]
    fn accepts_the_form_of_0_8_7() {
        let action = parse(
            "- action: git.clone\n  repository: https://github.com/comtrya/comtrya\n  reference: v0.8.7\n  directory: /tmp/comtrya\n",
        );

        assert_eq!("https://github.com/comtrya/comtrya", action.repo_url);
        assert_eq!(Some(String::from("v0.8.7")), action.reference);
    }
}
