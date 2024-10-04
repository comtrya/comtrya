# Privilege Escalation

## Escalating actions

Some actions may be run either privileged or unprivileged. For those unfamiliar, this would be by utilizing sudo or
running something as admin. A common action this is utilized with is [command.run](./command). Perhaps you, as the user,
wish to run a command that comtrya does not directly supply an action for, but that command required some form of
privilege escalation, some actions allow that to be specified. Here is an example utilizing `command.run`.

```
- action: command.run
  command: whoami
  sudo: true
```

Comtrya allows two terms for escalating privilege. Originally, it had to be done using sudo, however sudo is a term
typically tied to a specific application and has little meaning to a Windows user as an example. On unix-type systems,
there are also alternatives. Comtrya's architecture also allows for the ability to implement multiple providers for
privilege escalation. A more generic way to write the above action would be to use `privileged` in lieu of `sudo`.

```
- action: command.run
  command: whoami
  sudo: true
  privileged: true
```

## Privilege escalation providers

Comtrya, as of version 0.9.0 and later, will support different providers for privilege escalation. Not all operating systems may
support sudo or there is a preference for a different program to be used to escalate privileges, such as OpenBSD's doas
which has a port that runs on multiple platforms or run0 which is now a part of systemd starting with version 256 and
newer. 

In order to utilize different privilege providers, you must have a `Comtrya.yaml` file which contains configuration for
Comtrya. Below is an example.

```yaml
privilege: doas

variables:
  test: "one"
```

The `privilege` specifies which provider. Below are the providers and relevant values to be set for `privilege`.

| Provider | Value |
|----------|-------|
| Sudo     | sudo  |
| Doas     | doas  |
| Run0     | run0  |

As a note, Comtrya will always fall back to utilizing sudo. An example is when an action is being  executed, but no
privilege escalation provider is specified. 