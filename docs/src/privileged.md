# Privilege Escalation

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