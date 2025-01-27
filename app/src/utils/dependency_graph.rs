use std::{
    collections::HashMap,
    iter::from_fn,
    ops::Deref,
    sync::{Arc, LazyLock},
};

use anyhow::Result;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::{Reversed, Topo},
    Direction::{Incoming, Outgoing},
};
use tokio::sync::Barrier;

use super::ZipLongest;
use crate::Runtime;
use comtrya_lib::{
    actions::Actions::*,
    contexts::Contexts,
    manifests::Manifest,
    utilities::{get_privilege_provider, password_manager::PasswordManager},
};

type LockedManifest = Arc<LazyLock<Manifest, Box<dyn FnOnce() -> Manifest + Send>>>;
#[derive(Debug)]
pub struct DependencyGraph {
    pub(crate) graph: DiGraph<LockedManifest, ()>,
    pub(crate) name_to_idx: HashMap<String, NodeIndex>,
    pub(crate) barrier_map: HashMap<NodeIndex, Arc<Barrier>>,
}

impl DependencyGraph {
    pub async fn new(
        manifests: HashMap<String, Manifest>,
        contexts: &Contexts,
        runtime: &mut Runtime,
    ) -> Result<Self> {
        let mut this = Self {
            graph: DiGraph::new(),
            name_to_idx: HashMap::new(),
            barrier_map: HashMap::new(),
        };

        let mut first_package_install: Option<NodeIndex> = None;

        let mut should_ask_for_pass = false;
        for (_, manifest) in manifests.iter() {
            let to_index = this.add_manifest(manifest.clone()).await;

            for (dependant, action) in manifest.depends.iter().zip_longest(manifest.actions.iter())
            {
                if let Some(dependency_manifest) = dependant.and_then(|dep| manifests.get(dep)) {
                    let from_index = this.add_manifest(dependency_manifest.clone()).await;
                    this.graph.add_edge(from_index, to_index, ());
                }

                should_ask_for_pass = match action {
                    Some(PackageInstall(_)) | Some(PackageRepository(_)) => {
                        if let Some(index) = first_package_install {
                            first_package_install = first_package_install.or(Some(index));
                            this.graph.add_edge(index, to_index, ());
                        }
                        true
                    }
                    Some(CommandRun(cva)) => cva.action.privileged,
                    _ => false,
                };
            }
        }

        this.barrier_map.extend(
            this.graph
                .node_indices()
                .map(|node| {
                    (
                        node,
                        Arc::new(Barrier::new(
                            this.graph.neighbors_directed(node, Outgoing).count() + 1,
                        )),
                    )
                })
                .collect::<HashMap<_, _>>(),
        );

        if should_ask_for_pass {
            let mut password_manager =
                PasswordManager::new(get_privilege_provider(contexts).as_deref())?;
            password_manager.prompt("Please enter password:").await?;
            runtime.password_manager = Some(password_manager);
        }

        Ok(this)
    }

    fn get_ordered_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        let graph = Reversed(&self.graph);
        let mut topo = Topo::new(&graph);

        from_fn(move || topo.next(&graph))
    }

    pub fn get_ordered_manifests(&self) -> Vec<LockedManifest> {
        self.get_ordered_nodes()
            .flat_map(|idx| self.graph.node_weight(idx))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn add_manifest(&mut self, manifest: Manifest) -> NodeIndex {
        *self.name_to_idx.entry(manifest.get_name()).or_insert(
            self.graph
                .add_node(Arc::new(LazyLock::new(Box::new(|| manifest)))),
        )
    }

    pub fn get_barrier(&self, index: NodeIndex) -> &Arc<Barrier> {
        self.barrier_map.get(&index).unwrap()
    }

    #[allow(dead_code)]
    pub async fn get_node_index<M>(&self, manifest: M) -> &NodeIndex
    where
        M: AsRef<LazyLock<LockedManifest>> + Deref<Target = LazyLock<LockedManifest>>,
    {
        self.name_to_idx.get(&manifest.get_name()).unwrap()
    }

    #[allow(dead_code)]
    pub async fn get_successors(&self, manifest: &LockedManifest) -> Vec<NodeIndex> {
        self.graph
            .neighbors_directed(*self.get_node_from_manifest(manifest).await, Incoming)
            .collect()
    }

    pub async fn get_node_from_manifest(&self, manifest: &LockedManifest) -> &NodeIndex {
        // let join_set = JoinSet::new();
        self.name_to_idx.get(&manifest.get_name()).unwrap()
    }
}
