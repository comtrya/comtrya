# macOS

- macos.default

## macos.default

| Key    | Type   | Optional | Description                                                   |
|:-------|:-------|:---------|:--------------------------------------------------------------|
| action | string | no       | `macos.default`                                               |
| domain | string | no       | Domain: `defaults` or `domains` or https://macos-defaults.com |
| key    | string | no       | which key to change                                           |
| kind   | string | no       | value type                                                    |
| value  | string | no       | value                                                         |


### Example

```
- action: macos.default
  domain: com.apple.dock
  key: orientation
  kind: string
  value: left
  
- action: macos.default
  domain: com.apple.screencapture
  key: include-date
  kind: bool
  value: "false"

- action: macos.default
  domain: NSGlobalDomain
  key: "NSTableViewDefaultSizeMode"
  kind: int
  value: "1"
```
