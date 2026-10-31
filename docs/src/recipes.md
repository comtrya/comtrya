# Recipes

## Setting up a new machine with cloud-init

[cloud-init](https://cloud-init.io) only needs to do the bare minimum: install comtrya and run it. Your manifests do the rest.

```yaml
#cloud-config
users:
  - name: ada
    shell: /bin/bash
    # comtrya runs without a terminal here, so steps with `privileged: true` need sudo without a
    # password
    sudo: ALL=(ALL) NOPASSWD:ALL

packages:
  - curl
  - ca-certificates

runcmd:
  # Installs the latest release to /usr/local/bin. Set VERSION=v0.11.0 to pin one
  - curl -fsSL https://comtrya.dev/install.sh | sh
  # Runs the manifests as ada, not as root, so `user.home_dir` and friends are hers
  - su - ada -c 'comtrya -d https://github.com/ada/dotfiles apply'
```

- `runcmd` runs as root, so `install.sh` writes to `/usr/local/bin` without sudo. `INSTALL_DIR` puts the binary elsewhere.
- `-d` takes a git repository, also with a branch and a directory in it, like `https://github.com/ada/dotfiles#main:manifests`. See [CLI](./cli.md). comtrya clones it itself, so the image doesn't need git, and updates it on every run.
- To run only some manifests, add `-m`, e.g. `apply -m server.base`.

## Building software from a git clone

Some software has no package and has to be built from source. Clone it with [`git.clone`](./git.md), then run the build in the clone with [`command.run`](./command.md) and its `dir`:

```yaml
actions:
  - action: git.clone
    repo_url: https://github.com/example/tool
    reference: v1.2.3
    directory: "{{ user.home_dir }}/.cache/comtrya/src/tool"

  - action: command.run
    command: make
    args: [build]
    dir: "{{ user.home_dir }}/.cache/comtrya/src/tool"

  - action: command.run
    command: make
    args: [install]
    dir: "{{ user.home_dir }}/.cache/comtrya/src/tool"
    privileged: true
```

- `git.clone` skips a directory that exists already, and doesn't pull later changes. To build another version, change `reference` and `directory`, e.g. `.../src/tool-v1.2.4`.
- `command.run` runs on every apply. Keep the clone, so `make` only rebuilds what changed.
- `directory.remove` only removes empty directories. To delete a clone after the build, use `command.run` with `rm -rf` and the same path.
