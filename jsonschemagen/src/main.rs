//! Prints the JSON schema of manifests: version 1 by default, published as
//! comtrya.dev/schema.json, or version 2 with `--version 2`, published as
//! comtrya.dev/schema-v2.json

use comtrya_lib::manifests::Manifest;
use schemars::schema_for;

/// Version 2 is made from the version 1 schema
mod v2;

fn main() {
    let schema = serde_json::to_value(schema_for!(Manifest)).unwrap_or_default();
    let schema = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => schema,
        [flag, version] if flag == "--version" && version == "2" => v2::from_v1(schema),
        _ => {
            eprintln!("usage: jsonschemagen [--version 2]");
            std::process::exit(2);
        }
    };

    if let Ok(output) = serde_json::to_string_pretty(&schema) {
        println!("{output}");
    }
}
