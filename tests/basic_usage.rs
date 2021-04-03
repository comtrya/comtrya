use utils::*;
use tempdir::TempDir;

#[path = "./utils.rs"]
mod utils;

#[test]
fn prints_help() {
    let t = TempDir::new("comtrya").expect("could not create tempdir");
    let path = t.into_path();
    dir("directory", vec![
        dir("copy", vec![
            dir("files", vec![
                dir("mydir", vec![
                    f("file-a","some content a"),
                    f("file-b", "some other thing"),
                ])
            ]),
            f("main.yaml", r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#)
        ])
    ]).create_in(&path.clone()).expect("should have create stuff");



    let assert = cd(path).run("comtrya directory/copy/main.yaml --dry-run --no-color");

    assert.success().stdout(c(r#"INFO manifest_run{manifest="copy"}: Completed"#));

}
