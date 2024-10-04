# Commands

- command.run

## command.run

This is the most basic, but yet powerful action as it can be used to run anything needed that is not directly supported in comtrya by an action.

| Key        | Type   | Optional | Description                       |
|:-----------|:-------|:---------|:----------------------------------|
| action     | string | no       | command.run                       |
| command    | string | no       | command to run                    |
| args       | string | yes      | argument passed                   |
| dir        | string | yes      | actual working directory          |
| privileged | bool   | yes      | elevate privileges when executing |


### Example

```
- action: command.run
  dir: .
  command: echo
  args:
  - Hello world
```
