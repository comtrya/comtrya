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

## Manifests v2: variants override fields

In a version 1 manifest, a variant is a complete action: it has to repeat every field the action needs, like `command` above. In a [version 2](./manifests.md#manifests-v2-preview) manifest (a preview until 0.13.0), a variant sets only the fields that differ, and takes the others from its action:

```yaml
version: 2
actions:
  - action: package.install
    list: [fd, ripgrep]
    variants:
      - where: os.distribution == "Ubuntu"
        list: [fd-find, ripgrep]
        provider: aptitude
```

- **A field the variant sets replaces the action's field whole.** A list or a map isn't merged: `args: [c]` in a variant of an action with `args: [a, b]` gives `args: [c]`.
- **Which version applies is the same as in version 1:** the first variant whose `where` is true, else the action itself if its own `where` is true or missing. A variant doesn't take the action's `where`.
- **`title` and `continue_on_error` belong to the whole action**, whichever version applies. A variant can't set them, nor `action` or `variants`.
