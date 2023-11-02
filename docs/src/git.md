# Git

- git.clone

## git.clone

Perform a git clone on a repository from github.

| Key        | Type   | Optional | Description           |
|:-----------|:-------|:---------|:----------------------|
| action     | string | no       | `git.clone`           |
| repository | string | no       | repository to clone   |
| directory  | string | no       | directory to clone to |


### Example

```
- action: git.clone
  repository: comtrya/comtrya
  directory: {{ user.home_dir }}/Code/src/
```
