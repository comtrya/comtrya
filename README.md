# Comtrya

![Comtrya](/Comtrya.gif "Hello")

> This is better, yes?

---

[![License](https://img.shields.io/github/license/comtrya/comtrya?style=for-the-badge)](https://github.com/comtrya/comtrya/blob/main/LICENSE)

[![Latest GitHub Release](https://img.shields.io/github/v/release/comtrya/comtrya?label=Latest&style=for-the-badge)](https://github.com/comtrya/comtrya/releases/latest)
[![GitHub Actions Status](https://img.shields.io/github/actions/workflow/status/comtrya/comtrya/main.yaml?branch=main&style=for-the-badge)](https://github.com/comtrya/comtrya/actions/workflows/main.yml)

[![Codecov](https://img.shields.io/codecov/c/github/comtrya/comtrya?style=for-the-badge)](https://codecov.io/gh/comtrya/comtrya)

[![Discord](https://img.shields.io/discord/730728064031653999?label=Discord&style=for-the-badge)](https://rawkode.chat)

[![Packaging status](https://repology.org/badge/vertical-allrepos/comtrya.svg)](https://repology.org/metapackage/comtrya)

---

Want to learn how to use Comtrya? [Check the docs](https://comtrya.dev).

---

## About

Comtrya is a tool to help provision a fresh OS with the packages and configuration (dotfiles) you need to become productive again.

I'm a serial OS installer, I wipe the OS on my machines every, approx, 30 days. I've primarily relied on SaltStack to automate this, but I've grown frustrated with the mismatch between configuration management and personal provisioning.

I've also tried Ansible, Chef, Puppet, mgmt, and probably anything else you're about to suggest; they all have a flaw that makes it too cumbersome to adopt for the trivial use-case.

## Installing

You'll find binaries over on the [releases page](https://github.com/comtrya/comtrya/releases/latest).

If you're not feeling risk-averse, you can use this one-liner:

```shell
curl -fsSL https://get.comtrya.dev | sh
```

or specify `VERSION=vx.x.x` to pin to a release version

```shell
VERSION=v0.8.7 curl -fsSL https://get.comtrya.dev | sh
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

## What's Next?

You should take a look at the issues page to see what's available to contribute. Below is a short list of the major features that are upcoming.

### Better Output

Providing a `--quiet` or `--summary` option that restricts the output to the run time

```shell
Comtrya finished in 12.3s

Installed Packages: 12
Provisioned Files: 34
```

### Async DAG

We're using [petgraph](https://github.com/petgraph/petgraph) to build out the graph, but we're not traversing it in a way that will allow us to concurrently execute manifests at the same depth. This is something I wish to sort out pretty soon.

### Package Provider Enhancements

Currently, we execute arbitrary `packager install` commands. The provider spec should be enriched to support:

-   List refresh
-   Upgrades
-   Version pinning

### Integration tests

We are a bit light on tests at the moment, but we have started introducing some helpful plumbing in [tests](comtrya/app/tests).
