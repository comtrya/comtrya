# Binary

- binary.github

## binary.github

This action will grab a binary from GitHub and place it in the target directory, unless a file with that name is there already.

| Key        | Type   | Optional | Description                                        |
|:-----------|:-------|:---------|:---------------------------------------------------|
| action     | string | no       | binary.github                                      |
| name       | string | no       | name of binary locally after download              |
| directory  | string | no       | directory to save the binary locally               |
| repository | string | no       | GitHub repository, as `owner/repo`                 |
| version    | string | yes      | tag of the release; the latest release without it, or with `latest` |

Of the release's files, comtrya picks the one whose name matches this OS and CPU best, like `tool-aarch64-apple-darwin` on an Apple silicon Mac. Files for another OS are never picked. The action fails when no file fits.

When `GITHUB_TOKEN` is set, it's used for the GitHub API, which allows more requests per hour.

### Example

```yaml
- action: binary.github
  name: comtrya
  directory: /usr/local/bin
  repository: comtrya/comtrya
  version: v0.8.7
```
