# Group

- group.add

## Support

Not all systems are currently supported. The following is a list of group providers:

- FreeBSD
- Linux
- macOS

If your OS of choice is not listed, feel free to either contribute support by opening a pull request or requesting that support be added in the [repository's](https://github.com/comtrya/comtrya) issue tracker.

## group.add

Adds a group to the system.

| Key        | Type   | Optional | Description          |
|:-----------|:-------|:---------|:---------------------|
| action     | string | no       | `group.add`          |
| group_name | string | no       | name of group to add |


### Example

```yaml
actions:
  - action: group.add
    group_name: testgroup
```

