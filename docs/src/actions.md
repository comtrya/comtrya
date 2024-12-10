# Actions

As mentioned under the [Manifests](./manifests.md), actions are used to dictate an action that needs to be done to configuration a system. The following is an example of a simple action:

```
actions:
  # Action ID can be command.run or cmd.run
  - action: command.run
    command: echo
    args:
      - hi
```

The above does the following. It defines a list of actions. The action defined in a list of actions is the action for `command.run`, which runs the echo command, providing the argument of 'hi.' This results in something equivalent to running the following on a unix based system:

```
echo hi
```

## Groups of actions provided

Comtrya provides multiple actions which are broken down into groups with the actions being apart of a larger group.

- [Binary](./binary.md)
- [Commands](./command.md)
- [Files and Directories](./files-and-directories.md)
- [Git](./git.md)[^note]
- [Group](./group.md)
- [macOS](./macos.md)
- [Packages](./packages.md)
- [User](./user.md)

[^note]: Git actions are not avaible for 0.8.8 through 0.9.0, see action documentation for me information
