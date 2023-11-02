# Commands

- command.run

## command.run

This is the most basic, but yet powerful action as it can be used to run anything needed that is not directly supported in comtrya by an action.

| Key     | Type   | Optional | Description              |
|:--------|:-------|:---------|:-------------------------|
| action  | string | no       | command.run              |
| command | string | no       | command to run           |
| args    | string | no       | argument passed          |
| dir     | string | no       | actual working directory |


### Example

```
- action: command.run
  dir: .
  command: echo
  args:
  - Hello world
```
