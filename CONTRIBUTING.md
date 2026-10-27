# Welcome to Comtrya

This is a FOSS (free and open-source software) and lives from contributions of the community.

There are many ways to contribute

 * 📣 Spread the word about Comtrya
 * ✍️ Write tutorials, blog posts or share your manifests
 * 📝 Create or improve the [documentation](./docs)
 * 🐛 Submit [bug reports](https://github.com/comtrya/comtrya/issues/new?template=bug_report.md)
 * 💡 Share ideas and feature requests in [Discussions](https://github.com/comtrya/comtrya/discussions)
 * 🖥️ Test Comtrya on your OS and package manager (especially Windows and the BSDs)
 * 🏷️ Help triage issues: reproduce bugs, ask for missing details, find duplicates
 * 🧑‍💻 Contribute code or review pull requests


## 📜 Ground Rules

A community like this should be **open**, **considerate** and **respectful**. Please read our [Code of Conduct](./CODE_OF_CONDUCT.md).

Behaviours that reinforce these values contribute to a positive environment, and include:

 * **Being open**. Members of the community are open to collaboration.
 * **Acknowledging time and effort**. We're respectful and thoughtful when addressing the efforts of others, keeping in mind that often the work was done simply for the good of the community.
 * **Being respectful of differing viewpoints and experiences**. We're receptive to constructive comments and criticism, as the experiences and skill sets of other members contribute to the whole of our efforts.
 * **Gracefully accepting constructive criticism**. When we disagree, we are courteous in raising our issues.
 * **Using welcoming and inclusive language**. Anyone can participate and everyone can make a difference.


## 🧑‍💻 Code Contribution

You don't need any permissions to contribute code. Fork the repository, make your changes on a **new branch** in ***your*** fork and open a pull request.

1. Fork it (https://github.com/comtrya/comtrya/fork)
2. Create a new branch (`git checkout -b feat/foo-bar`)
3. Commit your changes (`git commit -am 'feat: add foo bar'`)
4. Push to the branch (`git push origin feat/foo-bar`)
5. Create a new pull request, our [pull request template](./.github/pull_request_template.md) guides you through it

Small, atomic commits make reviews easier and help track down regressions. We use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`, `docs:`, `chore:`, …) for commit messages and PR titles.

### Prerequisites

- Rust via [rustup](https://rustup.rs/), with `rustfmt` and `clippy`:
  - `rustup component add rustfmt clippy`
- The minimum supported Rust version is **1.88**. CI lints with it, so the code must build on it; tests run on stable.

### Before opening a pull request

Run `./check.sh` (or `check.ps1` on Windows). It
- formats your code with `cargo fmt`
- runs the linter `cargo clippy`, which fails on any warning, like CI
- runs all tests with `cargo test`

### Code style

Comtrya should be easy to read for someone who opens a file for the first time:

- **Small pieces.** Prefer short functions with a name that says what they do over long ones with nested `if`/`match`/loops. When a file grows past a few hundred lines, split it into modules.
- **No copies.** When two places do the same, share the code, for example a helper function or a common type for similar package providers.
- **Document everything,** private functions, structs and fields included: what it is for, and anything surprising. One line is often enough. Every module starts with a `//!` line that says what it contains.
- **Comments say why.** The code shows what happens; a comment explains why it has to be that way.
- **Errors, not panics.** Anything a user can cause, like a missing file or a typo in a manifest, ends in an error that says what's wrong. `unwrap()`/`expect()` are only for things that can't happen.
- **Refactors on their own.** Keep a pure clean-up in its own pull request, without changes in behaviour, so it's easy to review.

### Adding an action

An action is what a manifest asks for, like `file.copy`. It doesn't change the system itself: its `plan()` returns *steps*, and each step wraps an *atom*, the smallest change comtrya makes, like creating a directory or running a command. Comtrya checks every atom before it runs it, so an action that is already in place does nothing.

Note: the action and atom API is being reworked for 0.11.0 (see the `v0.11.0` milestone), so details below may change.

As an example, a `file.touch` action that creates an empty file:

1. **Write the action** in the group it belongs to, below [`lib/src/actions/`](./lib/src/actions), here `lib/src/actions/file/touch.rs`, and declare the module in the group's `mod.rs`. The fields are what users write in manifests; their doc comments become the descriptions in the JSON schema that editors show.

   ```rust
   //! `file.touch`: creates an empty file

   use crate::actions::Action;
   use crate::atoms::file::Create;
   use crate::contexts::Contexts;
   use crate::manifests::Manifest;
   use crate::steps::Step;
   use schemars::JsonSchema;
   use serde::{Deserialize, Serialize};
   use std::path::PathBuf;

   /// Creates an empty file, unless it exists
   #[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
   pub struct FileTouch {
       /// The file to create
       pub path: String,
   }

   impl Action for FileTouch {
       fn summarize(&self) -> String {
           format!("Creating file {}", self.path)
       }

       fn plan(&self, _manifest: &Manifest, _contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
           Ok(vec![Step::new(Create {
               path: PathBuf::from(&self.path),
           })])
       }
   }
   ```

2. **Reuse atoms.** [`lib/src/atoms/`](./lib/src/atoms) has atoms for files, directories, commands, downloads and git. A new atom implements the [`Atom`](./lib/src/atoms/mod.rs) trait: `plan()` says whether it needs to run, `execute()` makes the change, and `Display` describes the change in the output. `plan()` of an action and of an atom must never change the system, `--dry-run` relies on it.

3. **Register it** with one line in the `actions!` table in [`lib/src/actions/registry.rs`](./lib/src/actions/registry.rs). The line gives the name in manifests, any aliases, and the type; the `Actions` enum, the serde names, the schema and the output all come from it:

   ```rust
   FileTouch(FileTouch) = "file.touch";
   ```

   Every action also supports `where` and `variants`, nothing to do for that.

4. **Test it** next to the code: that a manifest entry deserializes into your action, and what `plan()` returns. Commands that only read the system, like asking a package manager what's installed, go through the [`Runner`](./lib/src/runner.rs) trait, so tests can answer them with `FakeRunner` instead of running them.

5. **Document it** for users in [`docs/src/`](./docs/src), in the page of its group, with a table of its fields and an example. A test fails for any registered action that the docs don't mention.

Don't use a new action in [`examples/`](./examples) yet: the [compatibility suite](#compatibility-with-existing-manifests) loads them with the last release too, which doesn't know the action.

### Compatibility with existing manifests

[`compat/`](./compat) loads manifests with your changes and with the last release of `comtrya-lib`, and fails if any manifest means something different, for example because an alias is gone, a field is read differently, or another variant applies on some system. If the difference is intended, record it in `known_differences` in `compat/tests/v1.rs` and mention it in the release notes. New manifest rules belong in `compat/fixtures/v1/`.

### Documentation

The documentation at [comtrya.dev](https://comtrya.dev) is built from the [`docs/`](./docs) folder with [mdBook](https://github.com/rust-lang/mdBook). To preview it locally:

```shell
cargo install mdbook
cd docs
mdbook serve
```


## 💬 Help

If you're stuck or have questions, join our [Matrix room](https://matrix.to/#/#comtrya:matrix.org) or open a [Discussion](https://github.com/comtrya/comtrya/discussions).

> See You Space Cowboy...
