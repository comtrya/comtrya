use comtrya_lib::manifests::Manifest;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(Manifest);

    if let Ok(output) = serde_json::to_string_pretty(&schema) {
        println!("{output}");
    }
}
