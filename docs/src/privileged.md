# Privilege Escalation

## Escalating actions

Some actions may be run either *privileged* or *unprivileged*. For those unfamiliar, this means for example utilizing sudo or running something as admin. A common action this may be required for is [command.run](./command). Perhaps you wish to run a command that comtrya does not directly supply an action for, but that command requires elevated privileges to make changes to the system. Some actions allow that to be specified. Here is an example making use of `command.run`:

```yaml
- action: command.run
  command: whoami
  sudo: true
```

Comtrya knows two keywords for escalating privilege. In older versions, it had to be done using sudo, however sudo is a term typically associated with a specific application and has little meaning to a Windows user for example. Even on Unix-like systems, there are alternatives available. Comtrya's architecture allows for using other providers for privilege escalation. A more generic way to write the above action would be to use `privileged` in lieu of `sudo`.

```yaml
- action: command.run
  command: whoami
  sudo: true
  privileged: true
```

## Privilege escalation providers

Comtrya, as of version 0.9.0 and later, supports different providers for privilege escalation. Not all operating systems may support sudo and some users prefer different programs, such as OpenBSD's doas which is available on multiple platforms or run0 which is part of systemd in versions 256 and newer.

In order to utilize different privilege providers, you must have a `Comtrya.yaml` file which contains configuration for Comtrya. Here is an example:

```yaml
privilege: doas

variables:
  test: "one"
```

The `privilege` specifies which provider. Below are the providers and relevant values to be set for `privilege`:

| Provider | Value |
|----------|-------|
| Sudo     | sudo  |
| Doas     | doas  |
| Run0     | run0  |

As a note, Comtrya will always fall back to utilizing sudo. An example is when an action is being  executed, but no privilege escalation provider is specified.
