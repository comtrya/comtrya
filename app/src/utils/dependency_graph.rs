use std::{collections::HashMap, iter::from_fn, ops::Deref, sync::Arc};

use anyhow::Result;
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::Topo,
    // Direction::*,
};
use tokio::sync::Mutex;

use super::ZipLongest;
use crate::Runtime;
use comtrya_lib::{
    actions::Actions::*,
    contexts::Contexts,
    manifests::Manifest,
    utilities::{get_privilege_provider, password_manager::PasswordManager},
};

#[derive(Debug)]
pub struct DependencyGraph {
    pub(crate) graph: DiGraph<Arc<Mutex<Manifest>>, ()>,
    pub(crate) name_to_idx: HashMap<String, NodeIndex>,
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
        };

        let mut first_package_install: Option<NodeIndex> = None;

        let mut should_ask_for_pass = false;
        for (_, manifest) in manifests.iter() {
            let from_index = this.add_manifest(manifest).await;

            for (dependant, action) in manifest.depends.iter().zip_longest(manifest.actions.iter())
            {
                if let Some(dependency_manifest) = dependant.and_then(|dep| manifests.get(dep)) {
                    let to_index = this.add_manifest(dependency_manifest).await;
                    this.graph.add_edge(from_index, to_index, ());
                }

                should_ask_for_pass = match action {
                    Some(PackageInstall(_)) | Some(PackageRepository(_)) => {
                        if let Some(index) = first_package_install {
                            first_package_install = first_package_install.or(Some(index));
                            this.graph.add_edge(from_index, index, ());
                        }
                        true
                    }
                    Some(CommandRun(cva)) => cva.action.privileged,
                    _ => false,
                };
            }
        }

        if should_ask_for_pass {
            let mut password_manager =
                PasswordManager::new(get_privilege_provider(contexts).as_deref())?;
            password_manager.prompt("Please enter password:").await?;
            runtime.password_manager = Some(password_manager);
        }

        Ok(this)
    }

    fn get_ordered_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        let mut topo = Topo::new(&self.graph);

        from_fn(move || topo.next(&self.graph))
    }

    pub fn get_ordered_manifests(&self) -> Vec<Arc<Mutex<Manifest>>> {
        self.get_ordered_nodes()
            .flat_map(|idx| self.graph.node_weight(idx))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub async fn add_manifest(&mut self, manifest: impl AsRef<Manifest>) -> NodeIndex {
        let manifest = manifest.as_ref();
        *self
            .name_to_idx
            .entry(manifest.get_name())
            .or_insert_with(|| self.graph.add_node(Arc::new(Mutex::new(manifest.clone()))))
    }

    #[allow(dead_code)]
    async fn get_node_index<M>(&self, manifest: M) -> &NodeIndex
    where
        M: AsRef<Mutex<Manifest>> + Deref<Target = Mutex<Manifest>>,
    {
        self.name_to_idx
            .get(&manifest.lock().await.get_name())
            .unwrap()
    }

    // #[allow(dead_code)]
    // async fn get_next_nodes(
    //     &self,
    //     manifest: Arc<Mutex<Manifest>>,
    // ) -> impl Iterator<Item = &Arc<Mutex<Manifest>>> {
    //     let futures: Vec<&Arc<Mutex<Manifest>>> = Vec::new();
    //     self.graph
    //         .neighbors_directed(*self.get_node_index(manifest).await, Outgoing)
    //         .map(|idx| self.graph.node_weight(idx).unwrap())
    //         .filter(|m| self.are_dependencies_completed(*self.get_node_index(Arc::clone(m))))
    // }

    // #[allow(dead_code)]
    // pub async fn get_ready_dependencies(
    //     &mut self,
    //     manifest: Arc<Mutex<Manifest>>,
    // ) -> Vec<&Arc<Mutex<Manifest>>> {
    //     manifest
    //         .lock()
    //         .await
    //         .depends
    //         .iter()
    //         .map(|dep| self.name_to_idx.get(dep).unwrap())
    //         .map(|idx| self.graph.node_weight(*idx).unwrap())
    //         .filter(|m| m.blocking_lock().state == ManifestState::Completed)
    //         .collect::<Vec<_>>()
    // }

    // #[allow(dead_code)]
    // async fn dependencies_completed(&self, node: NodeIndex) -> bool {
    //     self.graph
    //         .neighbors_directed(node, Incoming)
    //         .flat_map(|idx| self.graph.node_weight(idx))
    //         .all(|dep| dep.blocking_lock().state == ManifestState::Completed)
    // }

    pub async fn get_node_from_manifest(&self, manifest: &Arc<Mutex<Manifest>>) -> &NodeIndex {
        // let join_set = JoinSet::new();
        self.name_to_idx
            .get(&manifest.lock().await.get_name())
            .unwrap()
    }

    // #[allow(dead_code)]
    // pub fn are_dependencies_completed(&self, idx: NodeIndex) -> bool {
    //     self.graph.neighbors_directed(idx, Outgoing).all(|idx| {
    //         self.graph.node_weight(idx).unwrap().blocking_lock().state == ManifestState::Completed
    //     })
    // }

    // #[allow(dead_code)]
    // pub fn get_ready_manifests(&self) -> impl Iterator<Item = &Arc<Mutex<Manifest>>> {
    //     self.graph.node_weights().filter(|m| {
    //         m.blocking_lock().state == ManifestState::Pending
    //             && self.are_dependencies_completed(
    //                 *self.name_to_idx.get(&m.blocking_lock().get_name()).unwrap(),
    //             )
    //     })
    // }

    // #[allow(dead_code)]
    // pub fn all_manifests_completed(&self) -> bool {
    //     self.graph
    //         .node_weights()
    //         .all(|m| m.blocking_lock().state == ManifestState::Completed)
    // }

    // #[allow(dead_code)]
    // pub fn all_manifests_queued(&self) -> bool {
    //     self.graph
    //         .node_weights()
    //         .all(|m| m.blocking_lock().state != ManifestState::Pending)
    // }
}
