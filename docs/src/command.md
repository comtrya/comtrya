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

The command's output is logged line by line while it runs, visible with `-v`. Its stdin is empty, so a command that asks a question gets an end of file instead of waiting. A command that fails stops its action with its exit code and what it wrote to stderr, e.g. `Command failed with exit code: 1: …`; one ended by a signal says so, e.g. `Command was killed by signal 9`.

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

### Registering the output (manifest version 2)

In a [`version: 2`](./manifests-v2.md) manifest, `register: <name>` keeps what the command printed and how it exited. Later actions read it in templates and `where` conditions:

```yaml
version: 2
actions:
  - action: command.run
    command: rustc
    args: [--version]
    register: rustc

  - action: file.copy
    from: rust-1.8.toml
    to: "{{ user.home_dir }}/.config/rust.toml"
    where: '"1.8" in registered.rustc.stdout'
```

| Value                           | What it is                                                                 |
|:--------------------------------|:---------------------------------------------------------------------------|
| `registered.<name>.stdout`      | the lines it printed to stdout, joined with line breaks, without the last |
| `registered.<name>.stderr`      | the same for stderr                                                        |
| `registered.<name>.exit_code`   | 0, or the code it failed with when the action has `continue_on_error`      |

- The name is letters, digits and `_`, like `rustc_version`. Only `command.run` can `register`.
- The values are there once the action ran, for the later actions of its manifest and for the manifests that [depend](./dependencies.md) on it. With `--jobs`, other manifests may run before it or at the same time, so only dependents can rely on them.
- Reading a name that nothing registered fails the action: `` `registered.rustc` isn't set: no action before this one has `register: rustc`, or it didn't run ``. That's also the case when the command wasn't found, or when its `where` was false.
- An action whose fields read `registered.*` is read when it runs, so a missing or misspelled field in it is only reported then.
- `--dry-run` runs no commands, so nothing is registered: actions that read `registered.*` are skipped, with a line saying so.

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
