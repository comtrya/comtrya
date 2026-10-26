# Commands

- command.run

## command.run

This is the most basic, but yet powerful action as it can be used to run anything needed that is not directly supported in comtrya by an action. An alias exists, allowing you to abbreviate `command` as `cmd`.

| Key        | Type                    | Optional | Description                                     |
|:-----------|:------------------------|:---------|:------------------------------------------------|
| action     | string                  | no       | command.run                                     |
| command    | string                  | no       | command to run                                  |
| args       | string                  | yes      | argument passed                                 |
| dir        | string                  | yes      | actual working directory                        |
| privileged | bool                    | yes      | elevate privileges when executing               |
| env        | HashMap<string, string> | yes      | key and values for scoped environment variables |
| stdout     | string                  | yes      | file to write the command's output to           |

### Scoped environment variables

Sometimes, environment variables are needed to run a command or set of commands. `env` sets environment variables for the command of a single `command.run` action only; comtrya's own environment and later actions don't see them. A `PATH` in `env` is also where comtrya looks for the command. Multiple environment variables are supported.

Commands with `privileged: true` run through `sudo` (or the configured privilege provider), which usually resets the environment, so `env` may not reach them.

### Writing the output to a file

With `stdout`, the command's output is written to a file instead of the log, like `>` in a shell. This is handy for shell completions:

```yaml
- action: command.run
  command: just
  args:
    - --completions
    - bash
  stdout: "{{ user.home_dir }}/.local/share/bash-completion/completions/just"
```

- A relative path is relative to `dir`.
- Missing parent directories are created, and an existing file is overwritten.
- If the command fails, the file is left untouched.
- The file is written by comtrya itself, so with `privileged: true` it still belongs to the user running comtrya and its directory must be writable by them.

### Example

```yaml
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

# GOBIN isn't set here, env only applied to the command above
- action: command.run
  command: go
  args:
    - env
```
