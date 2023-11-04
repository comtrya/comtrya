# Introduction

![comtrya](https://777908980-files.gitbook.io/~/files/v0/b/gitbook-x-prod.appspot.com/o/spaces%2F-MWJSOygByBAd-VLa1iW%2Fuploads%2Fgit-blob-1a64ee32367f982956f49464b866a65d532625af%2FComtrya.gif?alt=media)

Comtrya is a tool that is built 100% in Rust. It's goal is to allow you, as the user, to provision and configure your systems using through the use of simple configuration files in the form of YAML files or TOML files.

The goals of comtrya are as follows:
- Run on any operating system
- Provide a simple YAML/TOML interface to, potentially, complex tasks

## Comparison to alternatives
 
### Ansible
Ansible is a great tool task runner, but comes with a lot of modules that aren't really necessary for localhost provisioning and can be cumbersome to run individual tasks within a playbook.

### SaltStack

SaltStack has been a favourite of mine (@rawkode) for many years, and while it's event system is a game changer for working with many devices - it's inability to display progress of large state runs makes it cumbersome to use for localhost provisioning.
