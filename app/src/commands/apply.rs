use std::{
    collections::HashMap,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread::available_parallelism,
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use comfy_table::{Cell, ContentArrangement, Table};
use petgraph::{
    stable_graph::{NodeIndex, StableDiGraph},
    Direction::*,
};
use rhai::Engine;
use tokio::task::JoinSet;
use tracing::{debug, error, info, info_span, instrument, trace, warn};

use super::ComtryaCommand;
use crate::{utils::ZipLongest, Runtime};
use comtrya_lib::{
    actions::Actions::{CommandRun, PackageInstall, PackageRepository},
    contexts::to_rhai,
    manifests::{load, Manifest},
    utilities::{get_privilege_provider, password_manager::PasswordManager},
};

#[derive(Parser, Debug, Clone)]
pub(crate) struct Apply {
    /// Run a subset of your manifests, comma separated list.
    /// This should be a list of manifest names. No paths.
    #[arg(short, long, value_delimiter = ',')]
    manifests: Vec<String>,

    /// Performs a dry-run without changing the system
    #[arg(long)]
    dry_run: bool,

    /// Define label selector
    #[arg(short, long)]
    pub label: Option<String>,
}

#[derive(Debug)]
struct DependenceyGraph {
    graph: StableDiGraph<Arc<Manifest>, ()>,
    name_to_idx: HashMap<Option<String>, NodeIndex>,
    should_ask_for_pass: bool,
}

impl DependenceyGraph {
    pub fn new(manifests: HashMap<String, Manifest>) -> Self {
        let mut this = Self {
            graph: StableDiGraph::new(),
            name_to_idx: HashMap::new(),
            should_ask_for_pass: false,
        };

        let mut first_package_install: Option<NodeIndex> = None;

        for (_, manifest) in manifests.iter() {
            let from_index = this.add_manifest(manifest);


            for (dependant, action) in manifest.depends.iter().zip_longest(manifest.actions.iter())
            {
                if let Some(dependency_manifest) = dependant.and_then(|dep| manifests.get(dep)) {
                    let to_index = this.add_manifest(dependency_manifest);
                    this.graph.add_edge(from_index, to_index, ());
                }

                if !this.should_ask_for_pass {
                    this.should_ask_for_pass = match action {
                        Some(CommandRun(cva)) => cva.action.privileged,
                        Some(PackageInstall(_)) | Some(PackageRepository(_)) => {
                            if let Some(index) = first_package_install {
                                this.graph.add_edge(from_index, index, ());
                            } else {
                                first_package_install = Some(from_index);
                            }
                            true
                        },
                        _ => false,
                    };
                }
            }
        }

        this
    }

    pub fn add_manifest(&mut self, manifest: &Manifest) -> NodeIndex {
        if let Some(&idx) = self.name_to_idx.get(&manifest.name) {
            idx
        } else {
            let m = Arc::new(manifest.to_owned());
            let idx = self.graph.add_node(m);
            self.name_to_idx.insert(manifest.name.to_owned(), idx);
            idx
        }
    }
}

impl Apply {
    fn manifest_path(&self, runtime: &Runtime) -> anyhow::Result<PathBuf> {
        for manifest in &self.manifests {
            if manifest.contains(std::path::MAIN_SEPARATOR) {
                return Err(anyhow!(
                    "Found a path, expected only names in the manifests list!"
                ));
            }
        }

        let first_manifest_path = runtime.config.manifest_paths.first().context(
            "No manifest paths found in config file, please add at least one path to your manifests"
        )?;

        let manifest_path = crate::manifests::resolve(first_manifest_path).context(format!(
            "Manifest location, {first_manifest_path:?}, could be resolved",
        ))?;

        trace!(manifests = self.manifests.join(",").deref());

        Ok(manifest_path)
    }

    #[instrument(skip(self, runtime))]
    pub async fn status(&self, runtime: &Runtime) -> anyhow::Result<()> {
        let contexts = &runtime.contexts;
        let manifest_path = self.manifest_path(runtime)?;

        println!("Load manifests from path: {manifest_path:#?}",);

        let manifests = load(manifest_path, contexts);

        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(40)
            .set_header(vec!["Manifest", "Count of Actions"]);

        for (name, manifest) in manifests.iter() {
            table.add_row(vec![
                Cell::new(name),
                Cell::new(format!("{}", manifest.actions.len())),
            ]);
        }

        println!("{table}");

        Ok(())
    }

    #[instrument(skip(self, runtime, manifest))]
    async fn process_manifest(
        &self,
        runtime: &Arc<Runtime>,
        manifest: &Arc<Manifest>,
    ) -> Result<()> {
        if let Some(label) = self.label.as_ref() {
            if !manifest.labels.contains(label) {
                info!(
                    message = "Skipping manifest, label not found",
                    label = label.as_str()
                );
                return Ok(());
            }
        }

        if let Some(where_condition) = &manifest.r#where {
            let engine = Engine::new();
            let mut scope = to_rhai(&runtime.contexts);
            
            match engine.eval_with_scope::<bool>(&mut scope, where_condition)
            {
                Ok(result) => {
                    debug!("Result of 'where' condition '{where_condition}' -> '{result}'",);
                }
                Err(err) => {
                    warn!("'where' condition '{where_condition}' failed: {err}");
                    info!("Skipping manifest, because 'where' conditions were false!");
                    return Ok(());
                }
            };
        }

        for action in manifest.actions.iter() {
            let span_action = info_span!("", %action).entered();
            let action = action.inner_ref();

            let mut steps = match action.plan(manifest, &runtime.contexts) {
                Ok(steps) => steps
                    .into_iter()
                    .filter(|step| step.do_initializers_allow_us_to_run())
                    .filter(|step| match step.atom.plan() {
                        Ok(outcome) => outcome.should_run,
                        Err(_) => false,
                    })
                    .peekable(),
                Err(err) => {
                    error!("Failed Processing: {manifest}. Action failed to get plan: {err:?}",);
                    return Ok(());
                }
            };

            if steps.peek().is_none() {
                info!("nothing to be done to reconcile action");
                span_action.exit();
                return Ok(());
            }

            if self.dry_run {
                return Ok(());
            }

            for mut step in steps {
                let password_manager = runtime.password_manager.clone();
                if let Err(err) = step.atom.execute(password_manager).await {
                    debug!("Atom failed to execute: {:?}", err);
                    break;
                }

                if !step.do_finalizers_allow_us_to_continue() {
                    debug!("Finalizers won't allow us to continue with this action");
                    break;
                }
            }
            info!("{}", action.summarize());
            span_action.exit();
        }

        info!("Completed: {manifest}",);
        Ok(())
    }
}

impl ComtryaCommand for Apply {
    #[instrument(skip(self, runtime))]
    async fn execute(&self, runtime: &mut Runtime) -> Result<()> {
        let ready_queue = Arc::new(crossbeam_queue::SegQueue::new());
        let done_queue = Arc::new(crossbeam_queue::SegQueue::new());
        let manifest_manager = DependenceyGraph::new(load(
            self.manifest_path(&runtime.clone())?,
            &runtime.contexts,
        ));

        let contexts = runtime.clone().contexts;
        if manifest_manager.should_ask_for_pass {
            let mut password_manager =
                PasswordManager::new(get_privilege_provider(&contexts).as_deref())?;
            password_manager.prompt("Please enter password:").await?;
            runtime.password_manager = Some(password_manager);
        }

        for idx in manifest_manager.graph.externals(Outgoing) {
            if let Some(manifest) = manifest_manager.graph.node_weight(idx) {
                ready_queue.push(manifest.clone());
            }
        }

        let shared_self = Arc::new(self.clone());
        let mut workers = JoinSet::new();
        let thread_runtime = Arc::new(runtime.clone());

        for _ in 0..available_parallelism().map_or(1, |p| p.get() / 2) {
            // Every runner needs it's own reference
            let thread_runtime = Arc::clone(&thread_runtime);
            let thread_ready_queue = Arc::clone(&ready_queue);
            let thread_done_queue = Arc::clone(&done_queue);
            let thread_self = Arc::clone(&shared_self);

            workers.spawn(async move {
                tokio::task::spawn_blocking(move || {
                    while let Some(manifest) = thread_ready_queue.pop() {
                        if let Err(e) = tokio::runtime::Handle::current().block_on(thread_self
                            .process_manifest(&thread_runtime, &manifest))
                        {
                            error!("Unable to process manifest: '{manifest}'. {e}")
                        } else {
                            thread_done_queue.push(manifest.clone());
                        }
                    }
                }).await?;
                Ok::<_, anyhow::Error>(())
            });
        }

        let shared_graph = Arc::new(RwLock::new(manifest_manager.graph));

        tokio::spawn(async move {
            if let Some(completed) = done_queue.pop() {
                if let Some(&idx) = manifest_manager.name_to_idx.get(&completed.name) {
                    {
                        match shared_graph.write() {
                            Ok(mut guard) => {
                                guard.remove_node(idx);
                            }
                            Err(e) => {
                                error!("Failed to acquire write lock: {e}");
                            }
                        }
                    }

                    match shared_graph.read() {
                        Ok(graph) => {
                            for idx in graph.externals(Outgoing) {
                                if let Some(manifest) = graph.node_weight(idx) {
                                    ready_queue.push(Arc::clone(manifest));
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to acquire read lock: {e}");
                        }
                    }
                }
            }
            // println!("{:?}", shared_graph.read().unwrap());
            drop(ready_queue);
        })
        .await?;

        workers.join_all().await;

        Ok(())
    }
}
