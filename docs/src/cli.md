# CLI

## Basic usage on local manifests

Comtrya works by running a manifest or set of manifests. The following are examples of running comtrya against manifests that are on the local machine.

```
# Run all manifests within your current directory
comtrya apply

# --manifests, or -m, will run a subset of your manifests
comtrya apply -m one,two,three

# Run all manifests within a specified directory
comtrya -d ./manifests apply
```

## Basic usage on remote manifests

Comtrya also has the ability to run remote manifests, normally hosted in a git repository on github.

```
# Manifests in a Git repository
comtrya -d https://github.com/rawkode/rawkode apply

# Manifests in a Git repository with a branch and path
comtrya -d https://github.com/rawkode/rawkode#main:dotfiles apply

# Manifests in a Git repository with a branch and path and a subset selector
comtrya -d https://github.com/rawkode/rawkode#main:dotfiles apply -m dev.git
```

## Help menu

Comtrya provides a help menu that can be shown by running the following in your terminal:

```
comtrya -h
```

```
USAGE:
    comtrya [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help        Prints help information
        --no-color    Disable color printing
    -V, --version     Prints version information
    -v, --verbose     Debug & tracing mode (-v, -vv)

OPTIONS:
    -d, --manifest-directory <manifest-directory>    Directory

SUBCOMMANDS:
    apply       Apply manifests
    contexts    List available contexts (BETA)
    help        Prints this message or the help of the given subcommand(s)
    version     Print version information
```
