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
