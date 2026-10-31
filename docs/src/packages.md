# Packages

- package.install
- package.repository

## Package Providers

Packages is a group of actions that utilize the local system's package manager, however it is a bit special. Unlike some other actions, packages can also contain *providers*. Some operating systems may have multiple package managers available and providers allow the user to choose which package manager to use. For example, macOS will automatically utilize homebrew as the default package manager, but this default can be overridden for another package manager on the system such as pkgin, which also supports macOS, or macports.

### Supported package providers

Not all package managers are supported. This is a list of currently supported package providers:

| Provider   | OS                |
|:-----------|:------------------|
| yay        | Arch (default, see below) |
| paru       | Arch              |
| pacman     | Arch              |
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

### Default package provider

Without `provider`, `package.install` and `package.repository` use the usual package manager of the OS, e.g. `apt` on Debian. To use another one for all of them, set `package_provider` in `Comtrya.yaml`:

```yaml
# Use pkgin instead of apt, unless an action sets its own provider
package_provider: pkgin
```

The order is:

1. `provider` on the action, or on its variant for this OS
2. `package_provider` in `Comtrya.yaml`
3. the default for this OS

The names and aliases are the same as for `provider`. An unknown name stops comtrya with an error listing the valid ones. The provider in use is available as `packages.provider` in templates and `where` conditions:

```yaml
where: packages.provider == "pkgin"
```

On an OS without a default, like Alpine, actions without a provider fail with an error until `provider` or `package_provider` is set.

### Arch Linux: pacman, yay and paru

On Arch and its derivatives, the default provider depends on how comtrya runs:

1. As root: `pacman`, because AUR helpers refuse to run as root. Packages from the AUR fail with pacman's "target not found".
2. Otherwise, `paru` if it is installed.
3. Otherwise `yay`. If yay isn't installed, comtrya installs `base-devel` and `git` with pacman and builds yay from the AUR.

paru and yay install packages from the official repositories and the AUR. They run as your user and ask for sudo themselves, so run comtrya as a normal user to install AUR packages. With `provider: yay` or `provider: paru` as root, `package.install` fails with an error saying so.

`pacman` installs packages from the official repositories only, with `pacman -S --needed`, and works as root too:

```yaml
- action: package.install
  provider: pacman
  list:
    - git
    - ripgrep
```

Before 0.10.0, `pacman` was another name for `yay`. Manifests that use `provider: pacman` for AUR packages need `provider: yay` now.

### Important note on homebrew and macOS

Some package manager providers can implement a `bootstrap` method that will automatically configure the package manager on the system if it is not part of the default installation. This is the case with macOS. Comtrya can automatically install `homebrew` to a macOS system and will do so if a manifest specifies a `package.install` action and does not override the macOS default of homebrew. On NetBSD, comtrya installs `pkgin` with `pkg_add pkgin` as root when it is missing; `pkg_add` needs to know where to download from, e.g. through `PKG_PATH` or `/etc/pkg_install.conf`.

## package.install

| Key        | Type   | Optional | Description                                                                        |
|:-----------|:-------|:---------|:-----------------------------------------------------------------------------------|
| action     | string | no       | `package.install`                                                                  |
| name       | string | no       | name of target package                                                             |
| list       | list   | yes      | list of multiple packages                                                          |
| provider   | string | yes      | Specify package provider                                                           |
| repository | string | yes      | Ignored with a warning, add repositories with `package.repository`                 |
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
| key         | string | yes      | Ignored with a warning                                    |
| fingerprint | string | yes      | Ignored with a warning, the key isn't checked against it  |

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
