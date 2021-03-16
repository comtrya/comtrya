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

If you're not feeling risk-adverse, you can use this one-liner:

```shell
curl -fsSL https://get.comtrya.dev | sh
```

If this doesn't work for your OS and architecture, please open an issue and we'll do our best to support it.

## Usage

```shell
comtrya <directory with manifests>
comtrya --help
```

## Manifests

A manifest is a collection of packages and files, and this will likely be expanded upon later. A Manifest is a logical grouping of resources that can be applied to your machine, and fetched from remote locations or local.

### Actions

Docs coming soon!

- directory.copy
- file.copy
- package.install

#### Packages

Single package:

```yaml
actions:
  - action: package.install
    name: git
```

Install several packages with the list version:

```yaml
actions:
  - action: package.install
    list:
      - git
      - minikube
```

#### Files

Files must be within a `files` directory within each manifest location.

```yaml
actions:
  - action: file.copy
    from: gitconfig
    to: /Users/rawkode/gitconfig
```

### Dependencies

```yaml
depends:
  - manifest_name
```

## What's Next?

You should take a look at the issues page (on GitLab, not GitHub) to see what's available to contribute. Below is a short list of the major features that are upcoming.

### Better Output

Currently, Comtrya `println!`'s pretty much everything. We need to use a proper logging library to restrict the scope of the output and should provide a summary output.

```shell
Comtrya finished in 12.3s

Installed Packages: 12
Provisioned Files: 34
```

### Async DAG

We're using [petgraph](https://github.com/petgraph/petgraph) to build out the graph, but we're not travesing it in a way that will allow us to concurrently execute manifests at the same depth. This is something I wish to sort out pretty soon.

### Config

TODO: Allow manifest directory and variables to be configured in a `Comtrya.yaml` file.

### Package Provider Enhancements

Currently, we execute arbitrary `packager install` commands. The provider spec should be enriched to support:

- Already installed lookups
- List refresh
- Upgrades
- Version pinning
- Packager installation
  - If Homebrew doesn't exist, offer to install it
  - If Nix doesn't exist, offer to install it
  - etc

### Files

Currently, files were prototyped pretty quickly. There's the ability to render and create files on a disk; we need to provide context providers to enrich the templating.

Also, symlinking only works if there's no rendering to take place; but there's no flag to encourage / enforce this.
