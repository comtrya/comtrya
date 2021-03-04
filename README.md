# Comtrya

![Comtrya](/Comtrya.gif "Hello")

> This is better, yes?

---

Comtrya is a tool to help provision a fresh OS with the packages and configuration (dotfiles) you need to become productive again.

I'm a serial OS installer, I wipe the OS on my machines every, approx, 30 days. I've primarily relied on SaltStack to automate this, but I've grown frustrated with the mismatch between configuration management and personal provisioning.

I've also tried Ansible, Chef, Puppet, mgmt, and probably anything else you're about to suggest; they all have a flaw that makes it too cumbersome to adopt for the trivial use-case.

## Getting Started

No binaries are distributed yet, please install and build with Cargo.

```shell
cargo install comtrya
comtrya --modules-directory ./dotfiles
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

We're using [petgraph]() to build out the graph, but we're not travesing it in a way that will allow us to concurrently execute manifests at the same depth. This is something I wish to sort out pretty soon.

### Config

The example [./dotfiles](./dotfiles) directory contains a [comtrya.yaml](./dotfiles/comtrya.yaml) file that should / will allow you to configure default providers, add contexts / variables, and specify the manifests directory. This doesn't exist yet :)

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
