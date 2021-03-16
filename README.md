# Comtrya

![Comtrya](/Comtrya.gif "Hello")

> This is better, yes?

---

## Warning

This tool is ridiculously early in its development. I'm building out features as I migrate my [dotfiles](https://gitlab.com/rawkode/rawkode) over.

Those dotfiles can be used as a "reference" for migrating your own.

Only for early adopters and masochists.

---
## About

Comtrya is a tool to help provision a fresh OS with the packages and configuration (dotfiles) you need to become productive again.

I'm a serial OS installer, I wipe the OS on my machines every, approx, 30 days. I've primarily relied on SaltStack to automate this, but I've grown frustrated with the mismatch between configuration management and personal provisioning.

I've also tried Ansible, Chef, Puppet, mgmt, and probably anything else you're about to suggest; they all have a flaw that makes it too cumbersome to adopt for the trivial use-case.

## Getting Started

You'll find binaries over on the [releases page](https://github.com/rawkode/comtrya/releases/latest).

If you're not feeling risk-averse, you can use this one-liner:

```shell
curl -fsSL https://get.comtrya.dev | sh
```

If this doesn't work for your OS and architecture, please open an issue and we'll do our best to support it.

## Usage

```shell
# Run all manifests within a directory
comtrya <directory with manifests>

# --manifests, or -m, will run a subset of your manifests
comtrya . -m one,two,three

# Show command usage
comtrya --help
```

## Manifests

A manifest is a collection of packages and files, and this will likely be expanded upon later. A Manifest is a logical grouping of resources that can be applied to your machine, and fetched from remote locations or local.

### Actions

- directory.copy
- file.copy
- package.install

#### Directories

Directories must be within a `files` directory within each manifest location.

```yaml
actions:
  - action: directory.copy
    from: includes
    to: /Users/rawkode/.zsh/
```

#### Files

Files must be within a `files` directory within each manifest location.

```yaml
actions:
  - action: file.copy
    from: gitconfig
    to: /Users/rawkode/gitconfig
    template: false # Whether to interpolate {{}} variable syntax with contexts
```
#### Packages

Currently Comtrya supports Homebrew and Aptitude as package providers. It'll detect, based on your system, which should be used.

This can be tweaked per action, using the `provider` parameter. Each provider also supports a `repository` parameter, which can add Homebrew Taps or PPAs for Ubuntu systems.

Single package:

```yaml
actions:
  - action: package.install
    name: git

  - action: package.install
    provider: homebrew
    repository: homebrew/cask
    name: docker

  - action: package.install
    provider: aptitude
    repository: ppa:longsleep/golang-backports
    name: golang-go
```

Install several packages with the list version:

```yaml
actions:
  - action: package.install
    list:
      - git
      - minikube
```
### Dependencies

Dependencies can be configured per manifest, not per action.

```yaml
depends:
  - manifest_name
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

We're using [petgraph](https://github.com/petgraph/petgraph) to build out the graph, but we're not travesing it in a way that will allow us to concurrently execute manifests at the same depth. This is something I wish to sort out pretty soon.

### Config

TODO: Allow manifest directory and variables to be configured in a `Comtrya.yaml` file. This will allow for `comtrya` with no arguments to function, as in the initial versions.

### Package Provider Enhancements

Currently, we execute arbitrary `packager install` commands. The provider spec should be enriched to support:

- List refresh
- Upgrades
- Version pinning
