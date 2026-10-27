//! The manifests and their `depends` as a graph, so dependencies run before the manifests that
//! need them.

use comtrya_lib::manifests::{LoadError, Loaded, Manifest};
use petgraph::{graph::NodeIndex, visit::DfsPostOrder, Graph};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{error, trace};

/// The manifests of a run and their dependencies.
///
/// Every manifest is a node. An edge points from a manifest to each manifest it depends on, and
/// from an unnamed root node to every manifest, so a walk from the root reaches all of them.
pub(super) struct ManifestGraph {
    /// Nodes are manifests, edges point to their dependencies
    pub(super) dag: Graph<Manifest, u32>,
    /// The unnamed node that every manifest hangs off
    root: NodeIndex,
    /// The node of each manifest, by its name
    nodes: HashMap<String, NodeIndex>,
    /// Manifests that fail before they run: they can't be loaded, or depend on a manifest that
    /// doesn't exist
    pub(super) broken: HashSet<NodeIndex>,
}

impl ManifestGraph {
    /// Builds the graph of the manifests loaded from `manifest_dir`.
    ///
    /// A manifest that can't be loaded fails like one whose action fails: it and everything that
    /// depends on it are skipped. It stays in the graph as an empty placeholder for that.
    pub(super) fn build(loaded: Loaded, manifest_dir: &Path) -> Self {
        let Loaded { manifests, errors } = loaded;

        let mut graph = ManifestGraph {
            dag: Graph::new(),
            root: NodeIndex::default(),
            nodes: HashMap::new(),
            broken: HashSet::new(),
        };
        graph.root = graph.dag.add_node(Manifest::default());

        for (name, manifest) in &manifests {
            graph.add_manifest(name, manifest);
        }
        for error in errors {
            graph.add_placeholder(error);
        }
        for (name, manifest) in &manifests {
            graph.add_dependencies(name, manifest, &directory_name(manifest, manifest_dir));
        }

        graph
    }

    /// Adds a node for the manifest `name`, below the root
    fn add_manifest(&mut self, name: &str, manifest: &Manifest) -> NodeIndex {
        let node = self.dag.add_node(manifest.clone());
        self.dag.add_edge(self.root, node, 0);
        self.nodes.insert(name.to_string(), node);
        node
    }

    /// Adds an empty, broken manifest in place of one that can't be loaded
    fn add_placeholder(&mut self, error: LoadError) {
        error!("{error}");

        let manifest = Manifest {
            name: Some(error.name.clone()),
            ..Default::default()
        };
        let node = self.add_manifest(&error.name, &manifest);
        self.broken.insert(node);
    }

    /// Adds an edge from the manifest `name` in `directory` to each of its dependencies. A
    /// dependency that doesn't exist makes the manifest broken
    fn add_dependencies(&mut self, name: &str, manifest: &Manifest, directory: &str) {
        let node = self.nodes[name];

        for dependency in &manifest.depends {
            let dependency = dependency_name(directory, dependency);

            match self.nodes.get(&dependency) {
                Some(&dependency_node) => {
                    trace!(
                        message = "Dependency Registered",
                        from = name,
                        to = dependency.as_str()
                    );
                    self.dag.add_edge(node, dependency_node, 0);
                }
                None => {
                    error!(
                        message = "Unresolved dependency",
                        manifest = name,
                        dependency = dependency.as_str()
                    );
                    self.broken.insert(node);
                }
            }
        }
    }

    /// The manifests to run, dependencies first: all of them, or the selected ones with what
    /// they depend on. Names that don't match a manifest are ignored
    pub(super) fn run_order(&self, selected: &[String]) -> Vec<NodeIndex> {
        // One walk for all starts: `move_to` keeps what's finished, so a manifest that several
        // selected ones depend on runs once
        let mut walk = DfsPostOrder::empty(&self.dag);
        let mut order = vec![];
        for start in self.starts(selected) {
            walk.move_to(start);
            while let Some(node) = walk.next(&self.dag) {
                order.push(node);
            }
        }
        order
    }

    /// Where the walk starts: the root for all manifests, or the selected ones
    fn starts(&self, selected: &[String]) -> Vec<NodeIndex> {
        if selected.is_empty() {
            return vec![self.root];
        }

        selected
            .iter()
            .filter_map(|name| self.nodes.get(name).copied())
            .collect()
    }

    /// Whether any manifest that `node` depends on is in `failed`
    pub(super) fn depends_on_any(&self, node: NodeIndex, failed: &HashSet<NodeIndex>) -> bool {
        self.dag
            .neighbors(node)
            .any(|dependency| failed.contains(&dependency))
    }
}

/// The directory of `manifest` below `manifest_dir`, written like a manifest name: `nested` for
/// `nested/main.yaml` and `nested/other.yaml`, empty for a top-level manifest
fn directory_name(manifest: &Manifest, manifest_dir: &Path) -> String {
    let Some(directory) = manifest
        .root_dir
        .as_deref()
        .and_then(|root_dir| root_dir.strip_prefix(manifest_dir).ok())
    else {
        return String::new();
    };

    directory
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join(".")
}

/// The full name of `dependency`, as written in the `depends` of a manifest in `directory`.
/// `./` stands for that directory: `./main` in `nested` is `nested.main`, at the top just `main`
fn dependency_name(directory: &str, dependency: &str) -> String {
    let prefix = match directory {
        "" => String::new(),
        directory => format!("{directory}."),
    };

    dependency.replace("./", &prefix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn manifest(name: &str, depends: &[&str]) -> Manifest {
        Manifest {
            name: Some(name.to_string()),
            depends: depends.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    fn graph(manifests: &[Manifest]) -> ManifestGraph {
        ManifestGraph::build(
            Loaded {
                manifests: manifests
                    .iter()
                    .map(|m| (m.name.clone().unwrap(), m.clone()))
                    .collect(),
                errors: vec![],
            },
            Path::new("/manifests"),
        )
    }

    /// A manifest in `directory` below `/manifests`
    fn manifest_in(directory: &str, name: &str, depends: &[&str]) -> Manifest {
        Manifest {
            root_dir: Some(Path::new("/manifests").join(directory)),
            ..manifest(name, depends)
        }
    }

    fn names(graph: &ManifestGraph, selected: &[&str]) -> Vec<String> {
        let selected: Vec<String> = selected.iter().map(|name| name.to_string()).collect();
        graph
            .run_order(&selected)
            .into_iter()
            .filter_map(|node| graph.dag[node].name.clone())
            .collect()
    }

    #[test]
    fn relative_dependencies_are_in_the_same_directory() {
        assert_eq!("nested.main", dependency_name("nested", "./main"));
        assert_eq!("a.b.c", dependency_name("a.b", "./c"));
        assert_eq!("other", dependency_name("nested", "other"));
        assert_eq!("main", dependency_name("", "./main"));
    }

    #[test]
    fn relative_dependencies_follow_the_manifest_file() {
        let graph = graph(&[
            // app.yaml, nested/main.yaml and nested/other.yaml
            manifest_in("", "app", &["./tools"]),
            manifest_in("", "tools", &[]),
            manifest_in("nested", "nested", &["./other"]),
            manifest_in("nested", "nested.other", &[]),
        ]);

        assert!(graph.broken.is_empty(), "{:?}", graph.broken);
        assert_eq!(vec!["tools", "app"], names(&graph, &["app"]));
        assert_eq!(vec!["nested.other", "nested"], names(&graph, &["nested"]));
    }

    #[test]
    fn dependencies_come_first() {
        let graph = graph(&[manifest("app", &["base"]), manifest("base", &[])]);

        assert_eq!(vec!["base", "app"], names(&graph, &["app"]));
    }

    #[test]
    fn a_shared_dependency_runs_once() {
        let graph = graph(&[
            manifest("a", &["base"]),
            manifest("b", &["base"]),
            manifest("base", &[]),
        ]);

        assert_eq!(vec!["base", "a", "b"], names(&graph, &["a", "b"]));
    }

    #[test]
    fn unknown_selected_names_are_ignored() {
        let graph = graph(&[manifest("app", &[])]);

        assert_eq!(1, graph.starts(&["app".into(), "nope".into()]).len());
        assert_eq!(vec![graph.root], graph.starts(&[]));
    }

    #[test]
    fn a_missing_dependency_breaks_the_manifest() {
        let graph = graph(&[manifest("app", &["nope"]), manifest("other", &[])]);

        assert_eq!(HashSet::from([graph.nodes["app"]]), graph.broken);
    }

    #[test]
    fn manifests_that_cant_be_loaded_are_broken_placeholders() {
        let graph = ManifestGraph::build(
            Loaded {
                manifests: HashMap::from([("app".to_string(), manifest("app", &["broken"]))]),
                errors: vec![LoadError {
                    name: "broken".to_string(),
                    path: PathBuf::from("broken.yaml"),
                    kind: comtrya_lib::manifests::LoadErrorKind::Parse("nope".to_string()),
                }],
            },
            Path::new("/manifests"),
        );
        let broken = graph.nodes["broken"];

        assert_eq!(HashSet::from([broken]), graph.broken);
        assert!(graph.dag[broken].actions.is_empty());
        assert!(graph.depends_on_any(graph.nodes["app"], &graph.broken));
    }
}
