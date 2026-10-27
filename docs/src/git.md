# Git

- git.clone

## git.clone

Clones a git repository. Nothing happens if `directory` already exists.

| Key        | Type   | Optional | Description                                                          |
|:-----------|:-------|:---------|:---------------------------------------------------------------------|
| action     | string | no       | `git.clone`                                                          |
| repo_url   | string | no       | URL of the repository to clone. `repository` works as well           |
| directory  | string | no       | directory to clone to                                                |
| reference  | string | yes      | branch or tag to check out, instead of the repository's default branch |

### Example

```yaml
actions:
  - action: git.clone
    repo_url: https://github.com/comtrya/comtrya
    directory: "{{ user.home_dir }}/Code/comtrya"

  - action: git.clone
    repo_url: https://github.com/comtrya/comtrya
    reference: v0.10.0
    directory: "{{ user.home_dir }}/Code/comtrya-0.10.0"
```

### History

comtrya 0.8.7 and earlier named the URL `repository`. `git.clone` was removed in 0.8.8 and came back in 0.9.1 with `repo_url` and without `reference`. Since 0.11.0, both names and `reference` work again, so manifests written for 0.8.7 run unchanged.
