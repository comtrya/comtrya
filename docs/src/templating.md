# Templating

Comtrya renders every manifest with [Tera](https://keats.github.io/tera/docs/), a template engine similar to Jinja2. Comtrya uses Tera 1.x.

Templates are rendered **before** the manifest is parsed, so you can use them anywhere in a manifest:

```yaml
actions:
  - action: file.link
    from: nvim
    to: "{{ user.config_dir }}/nvim"
```

Quote values that start with `{{`, otherwise YAML reads them as a mapping.

Files copied with `file.copy` and `template: true` are rendered the same way. See [Files and Directories](./files-and-directories.md#filecopy).

## Contexts

The values available in templates come from contexts. Run `comtrya contexts` to list them on your system.

| Context             | Contains                                                                                        |
|:--------------------|:------------------------------------------------------------------------------------------------|
| `user`              | `username`, `name`, `id`, `home_dir`, `config_dir`, `data_dir`, `data_local_dir`, `document_dir` |
| `os`                | `name`, `family`, `distribution`, `version`, `codename`, `edition`, `bitness`, `hostname`       |
| `env`               | All environment variables, e.g. `env.HOME`                                                      |
| `variables`         | `variables` from `Comtrya.yaml`, and values passed with `-D key=value`                         |
| `include_variables` | Variables loaded from the files listed under `include_variables` in `Comtrya.yaml`              |
| `privilege`         | `privilege`: the configured privilege escalation tool (`sudo`, `doas` or `run0`)                 |

### Including variables

`include_variables` in `Comtrya.yaml` loads more variables into the `include_variables` context:

```yaml
include_variables:
  - "file+yaml:///home/me/dotfiles/variables.yaml"
  - "file+toml:///home/me/dotfiles/variables.toml"
  - "dns+txt://example.com"   # TXT records of the form key=value
  - "op://Private/Dotfiles"   # fields of a 1Password item
```

`op://<vault>/<item>` reads a 1Password item with the [1Password CLI](https://developer.1password.com/docs/cli/get-started/), which must be installed and signed in (`op signin`, or the 1Password app integration). Every field with a value becomes a variable named after the field's label, e.g. `{{ include_variables.username }}`. Vault and item names may contain spaces. With several 1Password accounts, pick one with `?account=`, e.g. `op://Private/Dotfiles?account=my.1password.com`.

The values become plain template variables: Comtrya logs what an action does, so a secret used in a path or command can show up in its output. `comtrya contexts` lists their names, and prints the values only with `--show-values`. To put secrets into files without that, see `process_with` in [file.copy](./files-and-directories.md#filecopy).

Comtrya reads the includes at the same time. A variable from a later include overrides the same variable from an earlier one.
If one of the includes fails, for example because `op` isn't signed in, or hasn't answered after 30 seconds, Comtrya logs a warning and leaves out only that include's variables.

On Windows, the `user` directories use forward slashes, e.g. `C:/Users/me`. Windows accepts them, and they are safe inside double-quoted YAML strings. Other values, like `env.USERPROFILE`, still contain backslashes: put those in single quotes, e.g. `'{{ env.USERPROFILE }}'`, because YAML reads `\U` in double quotes as an escape.

## Filters and functions

All of Tera's [built-in filters](https://keats.github.io/tera/docs/#built-in-filters), [tests](https://keats.github.io/tera/docs/#built-in-tests) and [functions](https://keats.github.io/tera/docs/#built-in-functions) are available, as well as [control structures](https://keats.github.io/tera/docs/#control-structures) like `if` and `for`.

For example, to use only the major version of the OS:

```yaml
actions:
  - action: package.install
    where: os.distribution == "Fedora"
    list:
      - "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-{{ os.version | split(pat='.') | first }}.noarch.rpm"
```

Comtrya adds one function:

| Function             | Description                                                              |
|:---------------------|:-------------------------------------------------------------------------|
| `read_file_contents` | Returns the contents of the file at `path`, with surrounding whitespace trimmed |

```yaml
actions:
  - action: command.run
    command: echo
    args:
      - "{{ read_file_contents(path='/etc/hostname') }}"
```

## `where` is not Tera

The `where` conditions on manifests, actions and variants are not Tera templates. They are [Rhai](https://rhai.rs/book/) expressions, evaluated with the same contexts:

```yaml
where: os.name == "linux" && user.username == "rawkode"
```

See [Variants](./variants.md) for more examples.

## Errors

If a template can't be rendered, for example because it uses an unknown variable, Comtrya logs an error and skips the whole manifest.

That error is left out when the manifest's own `where` condition is false. A Windows-only manifest can use `get_env(name="LOCALAPPDATA")` without breaking on Linux or macOS. For this, the manifest must still be valid YAML or TOML before rendering, so quote values that contain `{{`.

To use a literal `{{` in a manifest, wrap it in Tera's `{% raw %}` block:

```yaml
actions:
  - action: command.run
    command: echo
    args:
      - "{% raw %}{{ not rendered }}{% endraw %}"
```
