# Files and Directories

- file.copy
- file.download
- file.link
- file.remove
- directory.copy

## file.copy

Action used to copy a file from one location to another.

| Key      | Type    | Optional | Description                           |
|:---------|:--------|:---------|:--------------------------------------|
| action   | string  | no       | `file.copy`                           |
| from     | string  | no       | source file                           |
| to       | string  | no       | destination file                      |
| template | boolean | yes      | renders files using context providers |
|          |         |          | default: `false`                      |
| chmod    | integer | yes      | octal permissions                     |


### Examples

```
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

```

## file.download

This action will download a file.

| Key    | Type   | Optional | Description      |
|:-------|:-------|:---------|:-----------------|
| action | string | no       | `file.download`  |
| from   | string | no       | source location  |
| to     | string | no       | destination file |

An alias also exists such that `source` can be used in lieu of `from` and `target` can be used in lieu of `to`.

### Example

```
actions:
  # This will be rendered with contexts
  - action: file.download
    from: https://google.com/robots.txt
    to: /tmp/google-robots.txt
```

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

```
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

```
- action: file.remove
  target: /tmp/some-file-rendered
```

## directory.copy

Copies a directory on the filesystem to another location.

| Key    | Type   | Optional | Description           |
|:-------|:-------|:---------|:----------------------|
| action | string | no       | `directory.copy`      |
| from   | string | no       | source directory      |
| to     | string | no       | destination directory |


### Example

```
- action: directory.copy
  from: managed_directory
  to: /root/location
```
