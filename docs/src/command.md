# Commands

- command.run

## command.run

This is the most basic, but yet powerful action as it can be used to run anything needed that is not directly supported in comtrya by an action.

| Key        | Type                    | Optional | Description                                     |
|:-----------|:------------------------|:---------|:------------------------------------------------|
| action     | string                  | no       | command.run                                     |
| command    | string                  | no       | command to run                                  |
| args       | string                  | yes      | argument passed                                 |
| dir        | string                  | yes      | actual working directory                        |
| privileged | bool                    | yes      | elevate privileges when executing               |
| env        | HashMap<string, string> | yes      | key and values for scoped environment variables |

### Scoped environment variables

Sometimes, environment variables are needed to run a command or set of commands. As of v0.9.1, Comtrya will offer the
ability to inject environment variables for the scope of a single `command.run` action. An initializer will run prior to
the action to inject the environment variables, then after the step as ran, a finalizer will remove those from the
environment. In the manifest, the environment is implemented as a hash map of keys and values. Multiple environment
variables are supported.

### Example

```
- action: command.run
  dir: .
  command: echo
  args:
  - Hello world
  
# we should see the GOBIN set in the go env output now
- action: command.run
  command: go
  args:
    - env
  env:
    GOBIN: /Users/test
    
# we should see that GOBIN is no longer set, showing that the variable has been cleaned up
- action: command.run
  command: go
  args:
    - env
```
