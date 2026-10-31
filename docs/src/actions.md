# Actions

As mentioned under [manifests](./manifests.md), actions are used to dictate an action that needs to be done to configuration a system. The following is an example of a simple action:

```yaml
actions:
  # Action ID can be command.run or cmd.run
  - action: command.run
    command: echo
    args:
      - hi
```

The above does the following: It defines a list of actions (with only one element). The action defined is the for `command.run`, which in this case runs the *echo* command, providing the argument of 'hi.' The result is something equivalent to running the following command line on a Unix-based system:

```shell
echo hi
```

## When an action fails

An action that fails stops its manifest: the actions after it don't run, the manifest fails, and the manifests that `depends` on it are skipped. Other manifests still run.

With `continue_on_error: true`, any action may fail without that. Its failure is shown as a warning, the manifest goes on with its next action, and it still counts as done, so the manifests that depend on it run too. Ctrl-C still stops the run.

```yaml
actions:
  # Fails on systems without a running Docker daemon, which is fine here
  - action: command.run
    command: docker
    args: [pull, alpine]
    continue_on_error: true
  - action: directory.create
    path: ~/.config/myapp
```

`continue_on_error` belongs to the action, not to one of its `variants`.

## Groups of actions provided

Comtrya provides multiple actions which are broken down into groups with the actions being apart of a larger group:

- [Binary](./binary.md)
- [Commands](./command.md)
- [Files and Directories](./files-and-directories.md)
- [Git](./git.md)[^note]
- [Group](./group.md)
- [macOS](./macos.md)
- [Packages](./packages.md)
- [User](./user.md)

[^note]: Git actions are not avaible for 0.8.8 through 0.9.0, see action documentation for me information
