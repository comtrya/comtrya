use std::{
    collections::HashMap,
    iter::from_fn,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::{Reversed, Topo},
    Direction::*,
};
use tracing::{error, trace};

use crate::Runtime;
use comtrya_lib::{
    actions::Actions,
    contexts::Contexts,
    manifests::{DependencyBarrier, Manifest},
    utilities::{get_privilege_provider, password_manager::PasswordManager},
};

type LockedManifest = Arc<LazyLock<Manifest, Box<dyn FnOnce() -> Manifest + Send>>>;

#[derive(Debug)]
pub struct DependencyGraph {
    pub(crate) graph: DiGraph<LockedManifest, ()>,
    pub(crate) name_to_idx: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    pub async fn new(
        mut manifests: HashMap<String, Manifest>,
        contexts: &Contexts,
        runtime: &mut Runtime,
    ) -> Result<Self> {
        let mut this = Self {
            graph: DiGraph::new(),
            name_to_idx: HashMap::new(),
        };
        let mut should_ask_for_pass = false;
        let mut dependency_map = Vec::new();

        for (_, manifest) in manifests.iter_mut() {
            manifest.barrier = Some(DependencyBarrier::new(manifest.depends.len() + 1));
            let node = this.add_manifest(manifest.clone()).await;
            this.name_to_idx.insert(manifest.get_name(), node);
        }

        for (node, manifest) in this.graph.node_indices().map(|n| (n, &this.graph[n])) {
            manifest.depends.iter().for_each(|dependency_name| {
                let name = manifest.get_name();

                let dep_prefix = name.rsplit_once('.').map(|(n, _)| n).unwrap_or(&name);
                let dependency_name = dependency_name.replace("./", &format!("{dep_prefix}."));

                let Some(dependency_manifest) = manifests.get(&dependency_name) else {
                    return error!(
                        message = "Unresolved dependency",
                        dependency = dependency_name
                    );
                };

                trace!(
                    message = "Dependency Registered",
                    from = name,
                    to = dependency_manifest.get_name()
                );

                dependency_map.push((node, this.name_to_idx[&dependency_manifest.get_name()]));
            });

            if !should_ask_for_pass {
                should_ask_for_pass = manifest.actions.iter().any(|action| match action {
                    Actions::CommandRun(cva) => cva.action.privileged,
                    Actions::PackageInstall(_) | Actions::PackageRepository(_) => true,
                    _ => false,
                });
            }
        }

        for (from, to) in dependency_map {
            this.graph.add_edge(from, to, ());
        }

        if should_ask_for_pass {
            debug!("Should be prompting for password. Asking now");

            let mut password_manager =
                PasswordManager::new(get_privilege_provider(contexts).as_deref())?;
            password_manager.prompt("Please enter password:")?;
            runtime.password_manager = Some(password_manager);
        }

        Ok(this)
    }

    fn ordered_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        let graph = Reversed(&self.graph);
        let mut topo = Topo::new(&graph);

        from_fn(move || topo.next(&graph))
    }

    pub fn get_ordered_manifests(&self) -> Vec<LockedManifest> {
        self.ordered_nodes()
            .flat_map(|idx| self.graph.node_weight(idx))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn add_manifest(&mut self, manifest: Manifest) -> NodeIndex {
        let idx = self.name_to_idx.entry(manifest.get_name()).or_insert(
            self.graph
                .add_node(Arc::new(LazyLock::new(Box::new(|| manifest)))),
        );
        *idx
    }

    pub async fn get_successors(&self, manifest: &LockedManifest) -> Vec<LockedManifest> {
        self.graph
            .neighbors_directed(self.get_node_from_manifest(manifest).await, Incoming)
            .map(|node| Arc::clone(&self.graph[node].clone()))
            .collect()
    }

    pub async fn get_node_from_manifest(&self, manifest: &LockedManifest) -> NodeIndex {
        // let join_set = JoinSet::new();
        *self.name_to_idx.get(&manifest.get_name()).unwrap()
    }
}
