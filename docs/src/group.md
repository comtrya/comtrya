# Group

- group.add

## Support

Not all systems are currently supported. The following is a list of group providers. If the system of your choice is not listed, feel free to either contribute support by opening a pull request or requesting that support be added in the [repository's](https://github.com/comtrya/comtrya) issue tracker.

- FreeBSD
- Linux
- macOS

## group.add

Adds a group to the system

| Key        | Type   | Optional | Description          |
|:-----------|:-------|:---------|:---------------------|
| action     | string | no       | `group.add`          |
| group_name | string | no       | name of group to add |


### Example

```
actions:
  - action: group.add
    group_name: testgroup
```

