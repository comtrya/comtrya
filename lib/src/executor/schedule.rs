//! Which manifest runs next: one after the other, or with `--jobs` several at once, each as soon
//! as the manifests it depends on are done

use super::graph::ManifestGraph;
use petgraph::graph::NodeIndex;
use std::collections::HashSet;
use std::sync::{Condvar, Mutex};

/// Calls `visit` for each manifest in `order`, see [`ManifestGraph::run_order`]. With one job in
/// that order, with more in up to `jobs` threads, a manifest only after its dependencies
pub(super) fn visit_all(
    graph: &ManifestGraph,
    order: Vec<NodeIndex>,
    jobs: usize,
    visit: impl Fn(NodeIndex) + Sync,
) {
    if jobs <= 1 {
        order.into_iter().for_each(visit);
        return;
    }

    let queue = Queue::new(graph, order);
    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                while let Some(node) = queue.next() {
                    let _done = Done {
                        queue: &queue,
                        node,
                    };
                    visit(node);
                }
            });
        }
    });
}

/// The manifests that haven't run yet
struct Queue<'a> {
    /// Where the dependencies come from
    graph: &'a ManifestGraph,
    /// What's left
    state: Mutex<State>,
    /// Woken when a manifest is done, so its dependents may be ready
    finished: Condvar,
}

/// The manifests that haven't run yet, and the ones that haven't finished
struct State {
    /// Not started, in run order
    waiting: Vec<NodeIndex>,
    /// Not started or still running. A manifest is ready when none of its dependencies is here
    unfinished: HashSet<NodeIndex>,
}

impl<'a> Queue<'a> {
    /// All of `order` waits
    fn new(graph: &'a ManifestGraph, order: Vec<NodeIndex>) -> Self {
        let unfinished = order.iter().copied().collect();
        Self {
            graph,
            state: Mutex::new(State {
                waiting: order,
                unfinished,
            }),
            finished: Condvar::new(),
        }
    }

    /// The next manifest whose dependencies are done, waiting for one if needed. `None` when
    /// all have started
    fn next(&self) -> Option<NodeIndex> {
        let mut state = self
            .state
            .lock()
            .expect("no worker panics while it holds the queue");
        loop {
            if state.waiting.is_empty() {
                return None;
            }
            if let Some(index) = state.waiting.iter().position(|&n| self.ready(&state, n)) {
                return Some(state.waiting.remove(index));
            }
            state = self
                .finished
                .wait(state)
                .expect("no worker panics while it holds the queue");
        }
    }

    /// Whether all manifests that `node` depends on are done
    fn ready(&self, state: &State, node: NodeIndex) -> bool {
        self.graph
            .dependencies(node)
            .all(|dependency| !state.unfinished.contains(&dependency))
    }

    /// `node` is done, its dependents may start
    fn finish(&self, node: NodeIndex) {
        let mut state = self
            .state
            .lock()
            .expect("no worker panics while it holds the queue");
        state.unfinished.remove(&node);
        self.finished.notify_all();
    }
}

/// Marks a manifest done when it's dropped, also when visiting it panics, so the other workers
/// don't wait for it forever
struct Done<'q, 'a> {
    /// The queue it came from
    queue: &'q Queue<'a>,
    /// The manifest
    node: NodeIndex,
}

impl Drop for Done<'_, '_> {
    fn drop(&mut self) {
        self.queue.finish(self.node);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::{Loaded, Manifest};
    use pretty_assertions::assert_eq;
    use std::path::Path;
    use std::time::Duration;

    fn graph(manifests: &[(&str, &[&str])]) -> ManifestGraph {
        let manifests = manifests
            .iter()
            .map(|(name, depends)| {
                let manifest = Manifest {
                    name: Some(name.to_string()),
                    depends: depends.iter().map(|d| d.to_string()).collect(),
                    ..Default::default()
                };
                (name.to_string(), manifest)
            })
            .collect();
        ManifestGraph::build(
            Loaded {
                manifests,
                errors: vec![],
            },
            Path::new("/manifests"),
        )
    }

    /// The names of the manifests as they start and end, with `jobs` workers. Each takes a
    /// moment, so independent ones overlap
    fn timeline(graph: &ManifestGraph, jobs: usize) -> Vec<String> {
        let events = Mutex::new(vec![]);
        visit_all(graph, graph.run_order(&[]), jobs, |node| {
            let Some(name) = graph.dag[node].name.clone() else {
                return;
            };
            events.lock().unwrap().push(format!("start {name}"));
            std::thread::sleep(Duration::from_millis(50));
            events.lock().unwrap().push(format!("end {name}"));
        });
        events.into_inner().unwrap()
    }

    #[test]
    fn one_job_runs_in_order() {
        let graph = graph(&[("app", &["base"]), ("base", &[])]);

        assert_eq!(
            vec!["start base", "end base", "start app", "end app"],
            timeline(&graph, 1)
        );
    }

    #[test]
    fn independent_manifests_run_at_the_same_time() {
        let graph = graph(&[("a", &[]), ("b", &[])]);

        let timeline = timeline(&graph, 2);

        assert!(
            timeline[..2].iter().all(|e| e.starts_with("start")),
            "{timeline:?}"
        );
    }

    #[test]
    fn a_manifest_waits_for_its_dependencies() {
        let graph = graph(&[("app", &["base", "tools"]), ("base", &[]), ("tools", &[])]);

        let timeline = timeline(&graph, 3);

        let start_app = timeline.iter().position(|e| e == "start app").unwrap();
        for dependency in ["end base", "end tools"] {
            let end = timeline.iter().position(|e| e == dependency).unwrap();
            assert!(end < start_app, "{timeline:?}");
        }
    }
}
