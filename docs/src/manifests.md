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

A manifest can state the format it is written for with a top-level `version` key. Version `1` is the default, and leaving the key out means the same; version `2` is a preview, see below. Comtrya refuses manifests with a newer version than it supports instead of misreading them. They fail like any other manifest that can't be loaded, see [When something fails](./dependencies.md#when-something-fails).

```yaml
version: 1
actions:
  - action: command.run
    command: echo
    args:
      - hi
```

### Manifests v2 (preview)

`version: 2` is a preview of the next manifest format, and can still change until comtrya 0.13.0. Don't use it for manifests you depend on yet. Comtrya 0.12.0 and older refuse it (0.9.x and older ignore `version` and misread it). So far it differs from version 1 in three ways:

- **Templates are rendered per value, after the manifest's `where`.** The file has to be valid YAML or TOML as it is, so quote values with templates (`path: "{{ user.home_dir }}/bin"`). A manifest whose `where` is false isn't rendered at all. `where` conditions aren't templates; they read the contexts themselves. An error names the value, like `actions[1].to`.
- **One name per action and field.** Old names like `cmd.run`, `dir.create`, `source`/`target` for `from`/`to` (in `file.link` too), or `sudo` for `privileged` are errors that name the one to use.
- **`title`** on an action is shown in the log instead of the generated summary:

```yaml
version: 2
actions:
  - action: package.install
    title: Shell tools
    list: [fd, ripgrep]
```

## Unknown keys

Comtrya warns about keys it doesn't read, like a misspelled field, and names the key it probably means:

```
WARN dev.yaml: `ags` in action 1 (command.run) is ignored, did you mean `args`?
```

Keys inside a field that holds an object, like `key` of `package.repository`, and keys in variants are checked too:

```
WARN repo.yaml: `ulr` in `key` of action 1 (package.repository) is ignored, did you mean `url`?
```

The manifest still runs without them. When the misspelled key is a required one, the manifest can't be loaded, and the error names the likely typo. A misspelled action name fails loading the same way, naming the closest action. Load errors give the action's position (counted from 0) and line:

```
actions[0]: unknown action `file.cpy`, did you mean `file.copy`? at line 2 column 5
```

## Which files are manifests

Every `.yaml`, `.yml` and `.toml` file below the manifest directory is a manifest, except:

- files in a `files/` directory, which hold the files that actions copy or link
- a `Comtrya.yaml` (or `Comtrya.yml`, `Comtrya.toml`) in the manifest directory, which is the config
- paths that match a glob in `ignore` in `Comtrya.yaml`

`ignore` is for YAML or TOML files that aren't manifests, like the config of an app kept next to its manifest:

```yaml
ignore:
  - zsh/plugins       # the directory and everything in it
  - "**/*.local.yaml" # in any directory
```

The globs match the path below the manifest directory, with `/` between directories on every system. `*` matches within one directory, `**` any number of directories, and a matching directory is left out with everything in it. An invalid glob, like `zsh/[a`, makes the config fail to load.

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
