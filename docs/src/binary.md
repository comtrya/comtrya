# Binary

- binary.github

## binary.github

This action will grab a binary from github and place it in a target directory.

| Key        | Type   | Optional | Description                           |
|:-----------|:-------|:---------|:--------------------------------------|
| action     | string | no       | binary.github                         |
| name       | string | no       | name of binary locally after download |
| directory  | string | no       | directory to save the binary locally  |
| repository | string | no       | Github repository                     |
| version    | string | no       | version/tag name                      |

### Example

```
- action: binary.github
  name: comtrya
  directory: /usr/local/bin
  repository: comtrya/comtrya
  version: v0.8.7
```
