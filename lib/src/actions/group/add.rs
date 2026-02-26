use super::Group;
use super::GroupVariant;
use crate::actions::Action;
use crate::contexts::Contexts;
use crate::manifests::Manifest;
use crate::steps::Step;
use std::ops::Deref;

pub type GroupAdd = Group;

impl Action for GroupAdd {
    fn summarize(&self) -> String {
        format!("Creating group {}", self.group_name)
    }

    fn plan(&self, _manifest: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        let variant: GroupVariant = self.into();
        let box_provider = variant.provider.clone().get_provider();
        let provider = box_provider.deref();

        let mut atoms: Vec<Step> = vec![];

        atoms.append(&mut provider.add_group(&variant, contexts));

        Ok(atoms)
    }
}
