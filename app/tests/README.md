# Integration tests

Files in this folder can test `comtrya` at a very high level.
The binary is built and then executed as a command. As such
things like `#[cfg(test)]` no longer kick in.
While it might be annoying, it does mean that we exercise `comtrya`
just as a user would!

## Setup

You will notice a `utils.rs` file that is not a binary. It's how we share
useful boilerplate code between the tests in this folder.
If you add a new file, you'll have to make to bring in `utils.rs` like so:

```rust
use utils::*;

mod utils;

#[test]
fn your_thing() {
}
```

that setup bring in `dir` and `f` function to crete a nested directory structures:

```rust
let t = TempDir::new("comtrya").expect("could not create tempdir");
let path = t.into_path();

dir(
    "directory",
    vec![dir(
        "copy",
        vec![
            dir(
                "files",
                vec![dir(
                    "mydir",
                    vec![
                        f("file-a", "some content a"),
                        f("file-b", "some other thing"),
                    ],
                )],
            ),
            f(
                "main.yaml",
                r#"
actions:
  - action: directory.copy
    from: mydir
    to: mydircopy
"#,
            ),
        ],
    )],
).create_in(&path.clone())
```

Once you have your tempdir setup as you want, you can change into and run `comtrya`:

```rust
let assert = cd(path).run("comtrya --help --no-color")
```

The resulting `assert` object is from [assert_cmd](https://docs.rs/assert_cmd/1.0.3/assert_cmd/) and can be used to assert on `stdout`, `stderr` and other bits.

Happy testing!


