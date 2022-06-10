use anyhow::anyhow;
use ignore::WalkBuilder;
use manifests::Manifest;
use petgraph::prelude::*;
use std::error::Error;
use std::fs::canonicalize;
use std::{collections::HashMap, ops::Deref};
use tera::Tera;
use tracing::{debug, error, info, span, trace, Level, Subscriber};
use tracing_subscriber::FmtSubscriber;

use crate::config::load_config;
use crate::contexts::{build_contexts, to_tera};

mod actions;
mod atoms;
mod commands;
mod config;
mod contexts;
mod manifests;
mod steps;
mod tera_functions;

use crate::manifests::get_manifest_name;
use crate::tera_functions::register_functions;

use commands::Args;

fn configure_subscriber(opt: &Args) -> impl Subscriber {
    let builder = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_ansi(!opt.no_color)
        .with_target(false)
        .without_time();

    match opt.verbose {
        0 => builder,
        1 => builder.with_max_level(Level::DEBUG),
        2 => builder.with_max_level(Level::TRACE),
        _ => builder.with_max_level(Level::TRACE),
    }
    .finish()
}

#[paw::main]
fn main(args: Args) -> anyhow::Result<()> {
    let subscriber = configure_subscriber(&args);

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let config = match load_config(args.clone()) {
        Ok(config) => config,
        Err(error) => {
            error!("{}", error.to_string());
            panic!();
        }
    };

    let manifest_path = match config.manifest_paths.first() {
        Some(l) => l,
        None => {
            return Err(anyhow!("No manifest location provided"));
        }
    };

    // Run Context Providers
    let contexts = build_contexts(&config);

    commands::execute(args, config)
}
