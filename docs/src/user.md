# User

- user.add
- user.group

## user.add

Adds a user to the system.

| Key      | Type   | Optional | Description               |
|:---------|:-------|:---------|:--------------------------|
| action   | string | no       | `user.add`                |
| username | string | no       | username of user          |
| fullname | string | yes      | full name of user         |
| home_dir | string | yes      | home directory of user, the tool's default without it |
| shell    | string | yes      | shell for user, the tool's default without it |
| group    | list   | yes      | groups to add new user to |

On Linux, macOS and FreeBSD, a user that exists already is left alone. The action fails when `username` is empty or the tool to create users (`useradd`, `dscl` or `pw`) isn't installed.


### Example

```yaml
- action: user.add
  fullname: testuser
  home_dir: /home/test
  username: test
  shell: sh

# Add user while also adding to groups
- action: user.add
  fullname: testuser
  home_dir: /home/test
  username: test
  shell: sh
  group:
  - testgroup
```

## user.group

Adds an already created user to a group.

| Key      | Type   | Optional | Description                      |
|:---------|:-------|:---------|:---------------------------------|
| action   | string | no       | `user.group`                     |
| username | string | no       | username of user to add to group |
| group    | list   | no       | groups to add user to            |

The action fails when `username` or `group` is empty, or the tool (`usermod`, `dscl` or `pw`) isn't installed.


### Example

```yaml
- action: user.group
  username: test
  group:
  - wheel
```
