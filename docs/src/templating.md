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

To use a literal `{{` in a manifest, wrap it in Tera's `{% raw %}` block:

```yaml
actions:
  - action: command.run
    command: echo
    args:
      - "{% raw %}{{ not rendered }}{% endraw %}"
```
