# Manifests

Comtrya provisions systems and performs configuration using a single or set of manifest files. These files are defined in either YAML or TOML syntax. Each manifest files is composed of [actions](./actions.md) and [dependencies](./dependencies.md). To break it down even further, each action is defined as an atom or a set of atoms. An action can be something as simple as echoing text out on a terminal. 

A dependency defines a relationship between manifests and actions. For instance, in order to configure neovim on a new system that is being provisioned, we might first want comtrya to ensure that neovim is installed. So we may define a dependency on an action to use the system's native package manager to install neovim before placing any configuration files we need for neovim. 

## Example of a YAML manifest

```
actions:
  - action: command.run
    command: echo
    args:
      - hi
```

## Example of TOML manifest

```
[[actions]]
action = "command.run"
command = "echo"
args = [ "hi" ]
```
