# Plugins

Plugins are created by the community to extend the functionality of comtrya. They allow users to customize and enhance their experience by adding new features or modifying existing ones.

Plugins can be used to automate tasks, integrate with other tools, or provide additional configuration options.

> [!IMPORTANT]
> Plugins are extremely powerful and can significantly alter the behavior of your system.
> Use them with caution and ensure you understand the implications of the changes they introduce.
> When using plugins, it's important to review their documentation and understand the changes they will make to your system.
> Always test plugins in a safe environment before deploying them in production to ensure they work as expected and do
> not introduce any unwanted side effects.

```yaml
actions:
  - action: plugin
    repo: username/repo
    version: *
    opts:
      action:
        key1: value1
        key2: value2
```

## Fields

- **Repository**: To use a repository, set the `repo` field to the GitHub repository in the format `username/repo`.
  - **repo**: The GitHub repository in the format `username/repo`. (other providers will be supported in the future)
  - **version**: The version of the plugin.
    - *Stable:*  Ommitting the version or explicitely statting as `"stable"` will use the most recent release.
    - *latest:* Set the version to `"*"` or explicitely state `"latest"` to use the most recent commit.
    - *Tagged:* Set the version to a specific tag, such as `"v1.2.3"` and that manifest will use the commit associated with that tag.

- **options**: A list of options for the plugin. (Aliases: `opts` and `spec`)
  - **tag**: A unique identifier for the plugin execution.
  - **Plugin specific!**: Additional configuration options specific to the plugin.
    - Refer to the plugin's documentation for available specifications.

## Examples

### Example 1: Using a Stable Version

```yaml
plugins:
  - source:
      repo: example_user/example_plugin
      version: stable # stable or omit for latest release
    options:
      - tag: configure
        setting: default
```

### Example 2: Using the Latest Version

```yaml
plugins:
  - source:
      repo: another_user/another_plugin
      version: latest # latest or *
    options:
      - tag: initialize
        mode: fast
```

### Example 3: Using a Tagged Version

```yaml
plugins:
  - source:
      repo: some_user/some_plugin
      version: v1.2.3 # must match tag or may not work
    options:
      - tag: deploy
        environment: production
```

## Additional Notes

- Ensure that the repository is accessible and the specified version exists.
- The `options` field allows you to customize the behavior of the plugin. Refer to the plugin's documentation for available options.

