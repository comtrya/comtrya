# Variants

All comtrya actions support the concept of variants. Variants allow you to modify how the action will be executed backed on contexts.

A command example of a variant is a variant based on the operating system. Suppose you have an action that is going to differ slightly between a linux, windows, and macOS system. The following is an example of that.

```yaml
actions:
  # This action has a "default" execution for when the variants don't overlay,
  # as it does not provide its own "where"
  - action: command.run
    command: echo
    args:
      - hello, world!
    variants:
      - where: os.name == "linux"
        command: reboot
      - where: os.name == "macos"
        command: echo
        args: ["Hello", "macOS"]
      - where: user.username == "rawkode"
        command: echo
        args: ["Hello", "rawkode!"]
```

Variants can can allow targeting an OS family or a specific distributions.

```yaml
actions:
  - action: command.run
    command: echo
    args:
      - hello, vanilla Linux!
    variants:
      - where: os.family == "unix"
        command: echo
        args: ["Hi,", "Unix"]
      - where: os.distribution == "Ubuntu"
        command: echo
        args: ["Hi,", "Ubuntu"]
      - where: os.bitness == "64-bit"
        command: echo
        args: ["Hi,", "64 bit!"]
```

Lastly, the `where` clause can be used to selectively skip or run tasks:

```yaml
actions:
  - action: command.run
    where: os.name == "linux"
    command: echo
    args:
      - Hello Linux
```
