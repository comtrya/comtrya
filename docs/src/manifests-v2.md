# Manifests v2 (preview)

`version: 2` is the next manifest format. It's a preview and can still change until comtrya 0.13.0, so don't use it yet for manifests you depend on. Version 1 manifests keep working, and both versions can sit in the same manifest directory.

Version 2 lets you write less and need fewer `command.run` workarounds. In short:

- [Templates are rendered per value](#templates-per-value), after the manifest's `where`, and a template can give a number or a boolean.
- [One name per action and field](#one-name-per-action-and-field): the old aliases are errors that name the one to use.
- [A variant sets only what differs](#variants-set-only-what-differs) from its action.
- [Files can sit next to the manifest](#files-next-to-the-manifest), without a `files/` directory.
- [`title`](#title) names an action in the log, and [`register`](#register) keeps what a command printed for later actions.
- [Package states](#package-states): `state: latest` and `state: absent` on `package.install` (also in version 1).

Comtrya 0.12.0 and older refuse `version: 2`; 0.9.x and older ignore `version` and misread such a manifest. [Migrating from version 1](./manifests-v2-migrating.md) lists what to change.

## Templates per value

Version 1 renders the whole file with Tera and then parses the result. Version 2 parses the file first, checks the manifest's `where`, and then renders each value on its own:

```yaml
{{#include ../examples/v2/templates.yaml}}
```

- **The file has to be valid YAML or TOML as it is**, so quote values with templates: `path: "{{ user.home_dir }}/bin"`.
- **A manifest whose `where` is false isn't rendered at all**, so a template that only works on Linux can't fail on macOS. `where` conditions aren't templates; they read the contexts themselves.
- **An error names the value it's in**, like `actions[1].args[0]: …`.
- **A value that is one template and nothing else becomes a number or a boolean** when its field doesn't hold text, like `privileged: "{{ variables.as_root }}"` above, or `chmod: "{{ variables.mode }}"`. Text fields keep the rendered text, so `args: ["{{ variables.port }}"]` stays `"8080"`.
- `{% if %}` and `{% for %}` work inside a value, but not around keys or actions: use `where` on the action, or a [variant](#variants-set-only-what-differs), instead.

## One name per action and field

Version 1 knows several names for some actions and fields. Version 2 knows one, and an old name is an error that names the one to use:

| Version 1 also takes                              | Version 2 only takes                            |
|:--------------------------------------------------|:------------------------------------------------|
| `cmd.run`                                         | `command.run`                                   |
| `dir.copy`, `dir.create`, `dir.remove`            | `directory.copy`, `directory.create`, `directory.remove` |
| `binary.gh`, `bin.github`, `bin.gh`               | `binary.github`                                 |
| `package.installed`, `package.repo`               | `package.install`, `package.repository`         |
| `source`, `target` in `file.copy`, `file.download`, `file.unarchive` and `file.link` | `from`, `to` |
| `sudo`                                            | `privileged`                                    |
| `repository` in `git.clone`                       | `repo_url`                                      |
| `url` in `package.repository`                     | `name`                                          |

The [schema](#editor-support) shows every action and field under its one name. Values aren't affected: provider names like `linux` or privilege providers like `doas` work as before.

## Variants set only what differs

In version 1, a variant is a complete action and repeats every field the action needs. In version 2, a variant sets only the fields that differ, and takes the others from its action:

```yaml
{{#include ../examples/v2/variants.yaml}}
```

- A field the variant sets replaces the action's field whole; lists and maps aren't merged.
- Which version applies is the same as in version 1: the first variant whose `where` is true, else the action itself.
- `title` and `continue_on_error` belong to the whole action; a variant can't set them.

More in [Variants](./variants.md#manifests-v2-variants-override-fields).

## Files next to the manifest

A source path that starts with `./` or `../` is relative to the manifest file, so a folder can hold a manifest and its files:

```text
manifests/
  zsh/
    zsh.yaml
    zshrc
```

```yaml
{{#include ../examples/v2/zsh/zsh.yaml}}
```

- A plain name, like `from: zshrc`, still looks in `files/`, as in version 1.
- This works for `file.copy`, `file.link`, `file.unarchive` and `directory.copy`.
- Comtrya reads every YAML and TOML file below the manifest directory as a manifest. If such a folder also holds YAML or TOML files that aren't manifests, like an app's own config, list them in [`ignore`](./manifests.md#which-files-are-manifests) in `Comtrya.yaml`.
- `{{ manifest.dir }}` and `{{ manifest.name }}` are the manifest's directory and name, in both versions.

## `title`

`title` on an action is shown in the log instead of the generated summary:

```yaml
{{#include ../examples/v2/title.yaml}}
```

## `register`

`register: <name>` on a `command.run` keeps what the command printed and how it exited. Later actions read it as `registered.<name>.stdout`, `.stderr` and `.exit_code`, in templates and in `where`:

```yaml
{{#include ../examples/v2/register.yaml}}
```

The values are there for the later actions of the manifest and for the manifests that depend on it. A dry run runs no commands, so it skips actions that read `registered.*`. All rules in [Registering the output](./command.md#registering-the-output-manifest-version-2).

## Package states

`state` on `package.install` says whether a package is installed (`present`, the default), installed and up to date (`latest`), or removed (`absent`). It works in version 1 too:

```yaml
{{#include ../examples/v2/packages.yaml}}
```

Which providers support it, and how they find outdated packages: [Packages](./packages.md#state).

## Editor support

Version 2 has its own JSON schema, published at `https://comtrya.dev/schema-v2.json`. It knows only the version 2 names, `title`, `register`, variants that set only some fields, and templates in place of numbers or booleans.

For YAML ([yaml-language-server](https://github.com/redhat-developer/yaml-language-server)):

```yaml
# yaml-language-server: $schema=https://comtrya.dev/schema-v2.json
version: 2
actions: []
```

For TOML ([Taplo](https://taplo.tamasfe.dev/)):

```toml
#:schema https://comtrya.dev/schema-v2.json
version = 2
actions = []
```
