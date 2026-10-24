# Manifests

Comtrya provisions systems and performs configuration using a single or set of manifest files. These files are defined in either YAML or TOML syntax. Each manifest files is composed of [actions](./actions.md) and [dependencies](./dependencies.md). To break it down even further, each action is defined as an atom or a set of atoms. An action can be something as simple as echoing text out on a terminal.

A dependency defines a relationship between manifests and actions. For instance, in order to configure neovim on a new system that is being provisioned, we might first want comtrya to ensure that neovim is installed. So we may define a dependency on an action to use the system's native package manager to install neovim before placing any configuration files we need for neovim.

## Example of a YAML manifest

```yaml
actions:
  - action: command.run
    command: echo
    args:
      - hi
```

## Example of TOML manifest

```toml
[[actions]]
action = "command.run"
command = "echo"
args = [ "hi" ]
```

## Editor support

A JSON schema for manifests is published at `https://comtrya.dev/schema.json`. Editors with a YAML or TOML language server can use it for completion and validation.

For YAML ([yaml-language-server](https://github.com/redhat-developer/yaml-language-server), e.g. the VS Code YAML extension), add this comment at the top of a manifest:

```yaml
# yaml-language-server: $schema=https://comtrya.dev/schema.json
actions:
  - action: command.run
    command: echo
    args:
      - hi
```

For TOML ([Taplo](https://taplo.tamasfe.dev/), e.g. the VS Code Even Better TOML extension):

```toml
#:schema https://comtrya.dev/schema.json
[[actions]]
action = "command.run"
command = "echo"
args = [ "hi" ]
```

To generate the schema for your installed version locally, run `cargo run -p jsonschemagen > schema.json` in a checkout of the repository.
