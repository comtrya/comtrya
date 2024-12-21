# User

- user.add
- user.group

## user.add

Adds a user to the system.

| Key      | Type   | Ooptional | Description               |
|:---------|:-------|:----------|:--------------------------|
| action   | string | no        | `user.add`                |
| fullname | string | no        | full name of user         |
| home_dir | string | no        | home directory of user    |
| username | string | no        | username of user          |
| shell    | string | yes       | shell for user            |
| group    | list   | yes       | groups to add new user to |


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


### Example

```yaml
- action: user.group
  username: test
  group:
  - wheel
```
