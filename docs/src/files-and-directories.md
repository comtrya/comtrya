# Files and Directories

- file.copy
- file.download
- file.link
- file.remove
- file.unarchive
- directory.copy
- directory.create
- directory.remove

All `directory.*` actions can also be written as `dir.*`, e.g. `dir.create`.

Every `file.*` and `directory.*` action takes `privileged: true` (or `sudo: true`) to make its change as root, e.g. for a file below `/etc`. See [Privilege Escalation](./privileged.md#files-and-directories).

## Note
The following commands expect the `from`/`source` to point to files/directories which are themselves under a "files" directory.
This is a restriction so that comtrya knows not to parse any .yaml file e.g. a config for a different tool as a manifest.
To see how it works check the [examples](https://github.com/comtrya/comtrya/tree/main/examples/file)

- file.link
- file.copy
- directory.copy


## file.copy

Action used to copy a file from one location to another.

| Key            | Type    | Optional | Description                           |
|:---------------|:--------|:---------|:--------------------------------------|
| action         | string  | no       | `file.copy`                           |
| from           | string  | no       | source file                           |
| to             | string  | no       | destination file                      |
| template       | boolean | yes      | renders files using context providers |
|                |         |          | default: `false`                      |
| chmod          | integer | yes      | octal permissions, e.g. `644` or `"0644"` |
|                |         |          | default: `644`                        |
| owned_by_user  | string  | yes      | user for chown                        |
| owned_by_group | string  | yes      | group for chown                       |
| passphrase     | string  | yes      | decrypts an [age](https://age-encryption.org) file encrypted with a passphrase |
| process_with   | list    | yes      | command that gets the file on stdin; its output is written instead, see below |


### Examples

```yaml
# Simple file copy
- action: file.copy
  from: procs-config.toml
  to: "{{ user.config_dir }}/procs/config.toml"

# With template and permissions
- action: file.copy
  from: managed_file
  to: /root/file
  template: true
  chmod: 644

# With encrypted file
- action: file.copy
  from: encrypted-file
  to: /tmp/some-decrypted-file
  passphrase: "1KZ2EXDHSQKZFQP43JK2LPXUFZ8D365CM5WQXRSH97U7N9WKRVFKS0TCS30"

# file copy with chown on unix systems
- action: file.copy
  from: procs-config.toml
  to: "{{ user.config_dir }}/procs/config.toml"
  owned_by_user: test
  owned_by_group: test
```

*Note: utilizing chown functionality will require running comtrya as root. Also, both a user and a group need to
be specified.*

### Seeing what changes

`comtrya apply --dry-run` shows what `file.copy` would change, without changing anything:

```text
 INFO execute:{manifest="git"}:{action=file.copy}: Contents of /home/me/.gitconfig change:
--- current
+++ new
@@ -1,3 +1,3 @@
 [user]
-  name = Old Name
+  name = New Name
   email = me@example.com
```

- For a file that doesn't exist yet or is empty, comtrya only says that it is created or written,
  without its contents.
- Permission changes from `chmod` get their own line.
- Binary files aren't diffed, comtrya only says that they change.
- Files written with `process_with` or `passphrase` are never shown, as they usually hold
  decrypted secrets. Rendered templates are shown, so a template that includes secrets from
  variables shows them too.

During a normal `apply`, the same diff is logged with `-v`.

### Processing files with a command

`process_with` runs a command, sends the source file to its stdin and writes its stdout to `to`.
Use it to decrypt secrets with the tool you already use: the encrypted file is the only copy in
your repository, and no encrypted copy is left behind next to the decrypted one.

- The first entry is the program, the rest are its arguments. No shell is involved, so `~` and
  `$VAR` are not expanded: use contexts like `{{ user.home_dir }}`, or `[sh, -c, "..."]`.
- The command runs while comtrya plans the action, also with `--dry-run`, so it must not change
  the system. It runs as your user, without `privileged`.
- If the output equals the current file, nothing is written.
- Its output is never shown in a diff, see [Seeing what changes](#seeing-what-changes).
- With `template: true`, the output is rendered as a template afterwards.
- If the command fails, the action fails and its stderr is shown.
- `passphrase` and `process_with` can't be combined.

```yaml
# GnuPG: uses your keyring and gpg-agent, so smartcards and YubiKeys work too
- action: file.copy
  from: credentials.fish.asc
  to: "{{ user.config_dir }}/fish/conf.d/credentials.fish"
  chmod: 600
  process_with: [gpg, --quiet, --decrypt]

# age with a key file (age1... or post-quantum age1pq1... keys)
- action: file.copy
  from: credentials.toml.age
  to: "{{ user.home_dir }}/.cargo/credentials.toml"
  chmod: 600
  process_with: [age, --decrypt, --identity, "{{ user.home_dir }}/.config/age/key.txt"]

# 1Password: the file contains secret references like {{ op://Private/GitHub/token }}.
# With more than one account signed in, pick one with --account
- action: file.copy
  from: gh-hosts.yml.tpl
  to: "{{ user.config_dir }}/gh/hosts.yml"
  chmod: 600
  process_with: [op, inject, --account, my.1password.com]

# sops
- action: file.copy
  from: secrets.yaml
  to: "{{ user.config_dir }}/app/secrets.yaml"
  chmod: 600
  process_with: [sops, --decrypt, --input-type, yaml, --output-type, yaml, /dev/stdin]
```

When gpg or age need a passphrase, they ask for it themselves: gpg through its pinentry, age on
the terminal.

## file.chown

This action will change the user and group owner of a file.

*Note: In order to utilize this, must run comtrya as root. Also both a user and group need to be specified.*

| Key    | Type    | Optional | Description                    |
|:-------|:--------|:---------|:-------------------------------|
| action | string  | no       | `file.chown`                   |
| path   | string  | no       | file to change ownership on    |
| user   | string  | no       | user to specify as file owner  |
| group  | string  | no       | group to specify as file owner |

### Examples

```yaml
actions:
  - action: file.chown
    path: ./files/some-file
    user: test
    group: test
```

## file.download

This action will download a file.

| Key            | Type   | Optional | Description      |
|:---------------|:-------|:---------|:-----------------|
| action         | string | no       | `file.download`  |
| from|source    | string | no       | source location  |
| to|target      | string | no       | destination file |
| chmod          | integer | yes     | octal permissions, e.g. `644` or `"0644"`, default: `644` |
| owned_by_user  | string | yes      | user for chown   |
| owned_by_group | string | yes      | group for chown  |
| sha256         | string | yes      | expected SHA-256 checksum of the file, as hex |

Without `sha256`, the file is downloaded once and kept as long as it exists. With `sha256`:

- the download fails if its checksum doesn't match, and an existing file is left untouched
- an existing file with a different checksum is downloaded again, so updating `from` and `sha256`
  together updates the file

GitHub shows the checksum of every release file on the release page, and many projects publish a
`SHA256SUMS` file. Otherwise, download the file once and run `shasum -a 256 <file>` (or
`Get-FileHash <file>` in PowerShell).

An alias also exists such that `source` can be used in lieu of `from` and `target` can be used in lieu of `to`.

### Example

```yaml
actions:
  # This will be rendered with contexts
  - action: file.download
    from: https://google.com/robots.txt
    to: /tmp/google-robots.txt

  # This whill also run a chown step on downloaded file
  - action: file.download
    from: https://google.com/robots.txt
    to: /tmp/google-robots.txt
    owned_by_user: nobody
    owned_by_group: nobody

  # Verified download, replaced when the checksum changes
  - action: file.download
    from: https://github.com/jtroo/kanata/releases/download/v1.12.0/linux-binaries-x64.zip
    to: "{{ user.cache_dir }}/kanata/linux-binaries-x64.zip"
    sha256: 0bedd91567c5d7c54679061baadc37e4f83fb71750003999bc1d11f2c9754f36
```

*Note: utilizing chown functionality will require running comtrya as root. Also, both a user and a group need to
be specified.*

## file.link

Create a symlink for files. This action can be used to symlink a single file or files in a directory.

| Key      | Type    | Optional | Description                       |
|:---------|:--------|:---------|:----------------------------------|
| action   | string  | no       | `file.link`                       |
| from     | string  | no       | symlink location                  |
| to       | string  | no       | symlink points to                 |
| walk dir | boolean | yes      | Walk diles in directory           |
|          |         |          | default: `false`                  |
| source   | string  | yes      | Used in conjunction with walk dir |
|          |         |          | in liue of `from`                 |


### Example

```yaml
# Symlink for a single file
- action: file.link
  from: /root/symlink
  to: managed_file

# Symlink for all files in a directory
- action: file.link
  source: walker
  target: /tml/walker-123
  walk_dir: true
```

## file.remove

Removes a file.

| Key    | Type   | Optional | Description        |
|:-------|:-------|:---------|:-------------------|
| action | string | no       | `file.remove`      |
| target | string | no       | file to be removed |


### Example

```yaml
- action: file.remove
  target: /tmp/some-file-rendered
```

## file.unarchive

This action provides the ability to unarchive a `tar.gz` file.

| Key    | Type   | Optional | Description                           |
|:-------|:-------|:---------|:--------------------------------------|
| action | string | no       | `file.unarchive`                      |
| from   | string | no       | full path to archive                  |
| to     | string | no       | destination of unarchived contents    |
| force  | bool   | yes      | force the unarchiving (defaults true) |

### Example

```yaml
actions:
  - action: file.download
    from: https://github.com/comtrya/comtrya/archive/refs/tags/v0.9.0.tar.gz
    to: /tmp/comtrya

  - action: file.unarchive
    from: /tmp/comtrya
    to: /tmp/
```

## directory.copy

Copies the contents of a directory into another directory, the same on all platforms. After the action, `to` holds everything `from` holds:
`to` and its subdirectories are created if missing, files with the same path are overwritten and other files in `to` are left alone.
A trailing `/` on `to` makes no difference.
On unix, symlinks inside `from` are copied as symlinks; on Windows the file they point to is copied.
The action fails if `to` is `from` itself or a directory inside it, like `cp` does.

Before 0.11.0, unix copied the directory itself to `<to>/<name>` unless `to` ended with `/`.
To keep that, put the name into `to`.

| Key    | Type   | Optional | Description           |
|:-------|:-------|:---------|:----------------------|
| action | string | no       | `directory.copy`      |
| from   | string | no       | source directory      |
| to     | string | no       | directory the contents of `from` are copied into |


### Example

```yaml
- action: directory.copy
  from: managed_directory
  to: /root/location
```

## directory.create

Creates a directory, including all missing parent directories. Does nothing if the directory already exists.

| Key    | Type   | Optional | Description         |
|:-------|:-------|:---------|:--------------------|
| action | string | no       | `directory.create`  |
| path   | string | no       | directory to create |

### Example

```yaml
- action: directory.create
  path: "{{ user.home_dir }}/.config/nvim"
```

## directory.remove

Removes a directory. Only **empty** directories are removed: if the directory isn't empty or doesn't exist, comtrya logs an error and skips the action.

| Key    | Type   | Optional | Description         |
|:-------|:-------|:---------|:--------------------|
| action | string | no       | `directory.remove`  |
| target | string | no       | directory to remove |

### Example

```yaml
- action: directory.remove
  target: /tmp/old-directory
```
