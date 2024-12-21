# Git

- git.clone

## Use notice

This functionality was removed in 0.8.8. However, it is back as of version 0.9.1. The older implementation had issues with openssl that caused some headaches. However, as of version 0.9.1, a new implementation will be available utilizing gix as the backend for git. There are some breaking changes.

## git.clone [0.9.1 and later]

Perform a git clone on a repository from github.

| Key        | Type   | Optional | Description           |
|:-----------|:-------|:---------|:----------------------|
| action     | string | no       | `git.clone`           |
| repo_url   | string | no       | repository to clone   |
| directory  | string | no       | directory to clone to |

### Example

```yaml
actions:
  - action: git.clone
    repo_url: https://github.com/comtrya/comtrya
    directory: /Users/test/Testing/comtrya/
```

## git.clone [0.8.7 and prior]

Perform a git clone on a repository from github.

| Key        | Type   | Optional | Description           |
|:-----------|:-------|:---------|:----------------------|
| action     | string | no       | `git.clone`           |
| repository | string | no       | repository to clone   |
| directory  | string | no       | directory to clone to |


### Example

```yaml
- action: git.clone
  repository: comtrya/comtrya
  directory: {{ user.home_dir }}/Code/src/
```
