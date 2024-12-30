# Files and Directories

- file.copy
- file.download
- file.link
- file.remove
- file.unarchive
- directory.copy

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
| chmod          | integer | yes      | octal permissions                     |
| owned_by_user  | string  | yes      | user for chown                        |
| owned_by_group | string  | yes      | group for chown                       |


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
| owned_by_user  | string | yes      | user for chown   |
| owned_by_group | string | yes      | group for chown  |

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

Copies a directory on the filesystem to another location.

| Key    | Type   | Optional | Description           |
|:-------|:-------|:---------|:----------------------|
| action | string | no       | `directory.copy`      |
| from   | string | no       | source directory      |
| to     | string | no       | destination directory |


### Example

```yaml
- action: directory.copy
  from: managed_directory
  to: /root/location
```
