# Packages

- package.install
- package.repository

## Package Providers

Packages is a group of actions that utilize the local system's package manager, however it is a bit special. Unlike some other actions, packages can also contain *providers*. Some operating systems may have multiple package managers available and providers allow the user to choose which package manager to use. For example, macOS will automatically utilize homebrew as the default package manager, but this default can be overridden for another package manager on the system such as pkgin, which also supports macOS, or macports.

### Supported package providers

Not all package managers are supported. This is a list of currently supported package providers:

| Provider   | OS                |
|:-----------|:------------------|
| pacman/yay | Arch              |
| paru       | Arch              |
| apt        | Debian/Ubuntu     |
| pkg        | FreeBSD           |
| pkgin      | NetBSD (Multiple) |
| brew       | macOS             |
| winget     | Windows           |
| xbps       | Void Linux        |
| zypper     | OpenSUSE          |
| macports   | macOS             |
| dnf        | Fedora            |
| snapcraft  | Linux             |

If you would like support to be added for a package provider, feel free to contribute the support or request support in the [repository](https://github.com/comtrya/comtrya) issue tracker.

### Important note on homebrew and macOS

Some package manager providers can implement a `bootstrap` method that will automatically configure the package manager on the system if it is not part of the default installation. This is the case with macOS. Comtrya can automatically install `homebrew` to a macOS system and will do so if a manifest specifies a `package.install` action and does not overridfe the macOS default of homebrew.

## package.install

| Key        | Type   | Optional | Description                                                                        |
|:-----------|:-------|:---------|:-----------------------------------------------------------------------------------|
| action     | string | no       | `package.install`                                                                  |
| name       | string | no       | name of target package                                                             |
| list       | list   | yes      | list of multiple packages                                                          |
| provider   | string | yes      | Specify package provider                                                           |
| repository | string | yes      | specific repository for a provider and package                                     |
| file       | bool   | yes      | Specify that package is a local package on the file system.                        |
|            |        |          | Default value is `false`                                                           |

### Example

```yaml
# Install package using default provider
- action: package.install
  name: curl

# Install a list of packages using default provider
- action: package.install
  list:
  - curl
  - wget

# Install a package using a specific package provider
- action: package install
  name: curl
  provider: pkgin

# Install a package specifying a repository
- action: package.repository
  provider: homebrew
  name: foo-repository/formulae
- action: package.install
  name: foo
  provider: homebrew
```

### Local package install support

Some package providers allow for installing a package from the local file system. An example of this would be `.pkg` files that can be installed using FreeBSD's package manager `pkg`. As of this time, it requires that the file property be set in the action's definition.

List of supported package providers:
- pkg (FreeBSD)
- aptitude (debian/ubuntu)

If you would like to have this feature supported on another package provider, please open an issue at the [comtrya GitHub repository](https://github.com/comtrya/comtrya).

#### Example

```yaml
- action: package.install
  name: /some/path/to/file/nano-8.1.pkg
  file: true
```

## package.repository

| Key      | Type          | Optional | Description                                     |
|----------|---------------|----------|-------------------------------------------------|
| name     | string        | no       | Alias of url                                    |
| key      | RepositoryKey | yes      | See table below                                 |
| provider | string        | yes      | Default value provided, specify package manager |

### RepositoryKey

| Key         | Type   | Optional | Description                                               |
|-------------|--------|----------|-----------------------------------------------------------|
| url         | string | no       | URL of the signing key                                    |
| name        | string | yes      | File name of the key, defaults to a hash of the URL      |
| key         | string | yes      |                                                           |
| fingerprint | string | yes      |                                                           |

### apt (Debian, Ubuntu)

For apt, `name` is the repository in the one-line format `<uri> <suite> [<component>...]`. Comtrya writes it as a [deb822](https://manpages.debian.org/stable/apt/sources.list.5.en.html) file to `/etc/apt/sources.list.d/comtrya-<uri>.sources` and runs `apt update`. If a `key` is given, it's downloaded to `/usr/share/keyrings/<name>.asc` and used as `Signed-By`.

```yaml
- action: package.repository
  name: "https://download.docker.com/linux/debian {{ os.codename }} stable"
  key:
    name: docker
    url: https://download.docker.com/linux/debian/gpg

- action: package.install
  name: docker-ce
```

On Ubuntu, `name` can also be a PPA such as `ppa:neovim-ppa/stable`, which is added with `add-apt-repository`.
