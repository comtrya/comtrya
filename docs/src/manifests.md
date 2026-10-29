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

## Manifest version

A manifest can state the format it is written for with a top-level `version` key. Only version `1` exists so far, and leaving the key out means the same. Comtrya refuses manifests with a newer version than it supports instead of misreading them. They fail like any other manifest that can't be loaded, see [When something fails](./dependencies.md#when-something-fails).

```yaml
version: 1
actions:
  - action: command.run
    command: echo
    args:
      - hi
```

## Unknown keys

Comtrya warns about keys it doesn't read, like a misspelled field, and names the key it probably means:

```
WARN dev.yaml: `ags` in action 1 (command.run) is ignored, did you mean `args`?
```

The manifest still runs without them. When the misspelled key is a required one, the manifest can't be loaded, and the error names the likely typo. A misspelled action name fails loading the same way, naming the closest action:

```
unknown action `file.cpy` in action 1, did you mean `file.copy`?
```

A `Comtrya.yaml` in the manifest directory is the config, not a manifest.

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
