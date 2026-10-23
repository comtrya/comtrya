# Comtrya

[![Latest GitHub Release](https://img.shields.io/github/v/release/comtrya/comtrya?label=Latest&style=for-the-badge)](https://github.com/comtrya/comtrya/releases/latest)
[![GitHub Actions Status](https://img.shields.io/github/actions/workflow/status/comtrya/comtrya/main.yaml?branch=main&style=for-the-badge)](https://github.com/comtrya/comtrya/actions/workflows/main.yaml)
[![Codecov](https://img.shields.io/codecov/c/github/comtrya/comtrya?style=for-the-badge)](https://codecov.io/gh/comtrya/comtrya)
[![License](https://img.shields.io/github/license/comtrya/comtrya?style=for-the-badge)](./LICENSE)
[![Matrix](https://img.shields.io/matrix/comtrya:matrix.org?style=for-the-badge)](https://matrix.to/#/#comtrya:matrix.org)

_Comtrya is a tool to help provision a fresh OS with the packages and configuration (dotfiles) you need to become productive again._

![Comtrya](/Comtrya.gif "Hello")

> This is better, yes?

[![Packaging status](https://repology.org/badge/vertical-allrepos/comtrya.svg)](https://repology.org/metapackage/comtrya)

---

Want to learn how to use Comtrya? [Check the docs](https://comtrya.dev).

Comtrya now also has documentation in the repository located in the `docs/` folder. In order to use it, please install
[mdbook](https://github.com/rust-lang/mdBook) using `cargo install mdbook`. Navigate to the `docs/` directory and run
`mdbook serve`. The documentation will be available through your web browser usually on `localhost:3000`.

---

## About

Comtrya was started by [@rawkode](https://github.com/rawkode), who explains why:

> I'm a serial OS installer, I wipe the OS on my machines every, approx, 30 days. I've primarily relied on SaltStack to automate this, but I've grown frustrated with the mismatch between configuration management and personal provisioning.
>
> I've also tried Ansible, Chef, Puppet, mgmt, and probably anything else you're about to suggest; they all have a flaw that makes it too cumbersome to adopt for the trivial use-case.

## Installing

You'll find binaries over on the [releases page](https://github.com/comtrya/comtrya/releases/latest).

If you're not feeling risk-averse, you can use this one-liner:

```shell
curl -fsSL https://get.comtrya.dev | sh
```

or specify `VERSION=vx.x.x` to pin to a release version

```shell
curl -fsSL https://get.comtrya.dev | VERSION=v0.9.2 sh
```

With a Rust toolchain installed, you can also build it from [crates.io](https://crates.io/crates/comtrya):

```shell
cargo install comtrya
```

If this doesn't work for your OS and architecture, please open an issue and we'll do our best to support it.

## Usage

```shell
# Run all manifests within your current directory
comtrya apply

# --manifests, or -m, will run a subset of your manifests
comtrya apply -m one,two,three

# Run all manifests within a specified directory
comtrya -d ./manifests apply
```

## 🏛️ License

Licensed under the [MIT license](./LICENSE).

## 🧩 Contribution

Comtrya is a free and open project and lives from contributions of the community.

See our [Contribution Guide](CONTRIBUTING.md)
