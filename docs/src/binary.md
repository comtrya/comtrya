# Binary

- binary.github

## binary.github

This action will grab a binary from GitHub and place it in the target directory.

| Key        | Type   | Optional | Description                           |
|:-----------|:-------|:---------|:--------------------------------------|
| action     | string | no       | binary.github                         |
| name       | string | no       | name of binary locally after download |
| directory  | string | no       | directory to save the binary locally  |
| repository | string | no       | Github repository                     |
| version    | string | no       | version/tag name                      |

### Example

```yaml
- action: binary.github
  name: comtrya
  directory: /usr/local/bin
  repository: comtrya/comtrya
  version: v0.8.7
```
