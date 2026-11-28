# Migrating from version 1

Version 1 manifests keep working, so there's no need to migrate all at once: move one manifest at a time, and keep the others as they are. [Manifests v2](./manifests-v2.md) is a preview until comtrya 0.13.0.

To move a manifest to version 2:

1. Add `version: 2` at the top. Switch the editor hint to `# yaml-language-server: $schema=https://comtrya.dev/schema-v2.json`, so the editor shows what's left.
2. Go through the table below.
3. Check with `comtrya apply -m <manifest> --dry-run`.

| Version 1 | Version 2 | Why |
|:----------|:----------|:----|
| An alias, like `cmd.run`, `dir.create`, `source`/`target` or `sudo` | The one name: `command.run`, `directory.create`, `from`/`to`, `privileged` (the [whole list](./manifests-v2.md#one-name-per-action-and-field)) | One way to write each thing |
| `path: {{ user.home_dir }}/bin` (unquoted) | `path: "{{ user.home_dir }}/bin"` | The file is parsed before templates are rendered, so it has to be valid YAML as it is |
| `{% if os.name == "linux" %}` around actions or keys | `where: os.name == "linux"` on the action, or a [variant](./manifests-v2.md#variants-set-only-what-differs) | Templates are rendered per value; `{% if %}` still works inside a value |
| `{% for %}` that writes actions | One action with a list, like `list:` on `package.install`, or one action each | Templates can't write keys or actions |
| `chmod: {{ variables.mode }}` | `chmod: "{{ variables.mode }}"` | A value that is one template still becomes a number |
| A variant that repeats every field of its action | A variant with only the fields that differ | Variants [take the rest](./manifests-v2.md#variants-set-only-what-differs) from their action |
| `files/zshrc` next to `zsh.yaml`, `from: zshrc` | `zshrc` next to `zsh.yaml`, `from: ./zshrc` | [Files next to the manifest](./manifests-v2.md#files-next-to-the-manifest); a plain name still looks in `files/` |
| An app's YAML config inside `files/` to keep it from being read as a manifest | Next to the manifest, listed in [`ignore`](./manifests.md#which-files-are-manifests) | Only needed once files leave `files/` |
| `command.run` writing a file that a later `command.run` reads back | `register` on the first, `{{ registered.<name>.stdout }}` in the later one | [`register`](./manifests-v2.md#register) |

`where` conditions don't change: they were Rhai before and stay Rhai. In version 1 they were also rendered as templates along with the whole file; in version 2 they aren't, so a `where` with `{{ … }}` in it has to read the context directly, like `user.name == "ada"`.

Other fields, contexts, `depends`, `labels` and the actions themselves are the same in both versions.
