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
