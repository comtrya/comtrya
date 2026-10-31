# Commands

Comtrya offers several commands for use. They can be listed in the terminal by running comtrya with the help command.

```shell
comtrya help
```

The most frequently used command s the **apply** command, which will apply the actions of provided manifests to the system.

## Available Commands

| Command         | Description                                  |
|:----------------|:---------------------------------------------|
| apply           | Apply manifests                              |
| status          | List manifest status                         |
| version         | Print version information                    |
| contexts        | List available contexts                      |
| gen-completions | Auto generate completions                    |
| help            | Print out help information for using comtrya |

## Apply

The apply command executes and runs the manifests. There are a few ways to do this.

The first option is to point comtrya to a directory of manifests and have it execute them all:

```shell
comtrya -d ./manifests apply
```

As shown, this is done with the `-d` option, which tells comtrya the directory that contains the manifests to be applied.

The second option is to specify specific manifest(s) to be executed:

```shell
comtrya -d ./ apply -m one,two,three
```

The `-m` option is used to let comtrya know which manifests to apply. Note that the name of the manifest (i.e. one.yaml) is only the name and must not contain any path information or file extension (.yaml). So, `/manifests/one` is not a valid input. Any manifests are expected to be located in the directory of the manifests you specified.

Suppose you have a directory `manifests/` that contains the manifests `one.yaml` and `two.yaml`. You want to *only* execute `one.yaml`. There are three ways to achieve this. You can simply specify the manifest if it's in the current working directory:

```shell
cd manifests/
comtrya -d ./ apply -m one
```

Or you can specify the directory:

```shell
comtrya -d ./manifests/one.yaml apply
```

Alternatively a combination of the two is possible as well:

```shell
comtrya -d ./manifests/ apply -m one
```

### Dry run

`--dry-run` plans every action without changing the system. For `file.copy`, it shows a diff of what
would change in each file; see [Files and Directories](./files-and-directories.md#seeing-what-changes).

An action that can't be planned, like a `file.copy` whose source is missing, fails the dry run
like it fails a real one: its manifest and the manifests that depend on it count as failed, and
comtrya exits with a non-zero status. Unlike a real run, the dry run reports every action of a
manifest that can't be planned, not only the first one.

```shell
comtrya -d ./manifests apply --dry-run
```

### Several manifests at once

`--jobs N` (or `-j N`) runs up to N manifests at the same time, each as soon as the manifests it
`depends` on are done. The actions and steps of one manifest still run in order. Steps of
different manifests that change the same file or directory, use the same package manager, or
may ask for a password take turns; a `command.run` takes turns with every other step, because
comtrya can't know what it changes. The default is 1, one manifest after the other. A dry run
always runs one manifest after the other.

Log lines of manifests that run at once can come between each other; each line names its
manifest and action.

```shell
comtrya -d ./manifests apply --jobs 4
```

### More output

With `-v`, `apply` also shows what each step changes, and the output of programs that `command.run` and package managers run, line by line while they run. `-vv` shows even more.

### Keeping a log

`--log-file <path>` writes the log to a file as well, for reading later. The file gets times and no colors, and always as much as `-v` shows, or more with `-vv`; the terminal shows what it always does. A run adds to the end of the file.

```shell
comtrya --log-file ~/comtrya.log -d ./manifests apply
```

### Stopping a run

Ctrl-C lets the step that is running finish, then stops: nothing starts after it, and comtrya exits with an error. Press Ctrl-C again to quit at once. A program that is running gets the Ctrl-C from the terminal too, and usually stops, which fails its step.

## Contexts

The **contexts** command is useful to see what comtrya knows about your system. This can be environment variables, included variables, information about the OS, user information and other variables. Below is an exmaple of the output.

```text
env
 COLORTERM     DBUS_SESSION  DESKTOP_SES  DISPLAY      DOTNET_BUND  GDMSESSION
               _BUS_ADDRESS  SION                      LE_EXTRACT_
                                                       BASE_DIR
 GNOME_DESKTO  GNOME_SHELL_  GNOME_TERMI  GNOME_TERMI  GPG_AGENT_I  GTK_IM_MODU
 P_SESSION_ID  SESSION_MODE  NAL_SCREEN   NAL_SERVICE  NFO          LE
 GTK_MODULES   HOME          LANG         LESSCLOSE    LESSOPEN     LOGNAME
 LS_COLORS     NVM_BIN       NVM_CD_FLAG  NVM_DIR      NVM_INC      OLDPWD
                             S
 PATH          PWD           QT_ACCESSIB  QT_IM_MODUL  SESSION_MAN  SHELL
                             ILITY        E            AGER
 SHLVL         SSH_AGENT_LA  SSH_AUTH_SO  SWIFTLY_BIN  SWIFTLY_HOM  SYSTEMD_EXE
               UNCHER        CK           _DIR         E_DIR        C_PID
 TERM          TEXTDOMAIN    USER         USERNAME     VTE_VERSION  WINDOWPATH
 XAUTHORITY    XDG_CONFIG_D  XDG_CURRENT  XDG_DATA_DI  XDG_MENU_PR  XDG_RUNTIME
               IRS           _DESKTOP     RS           EFIX         _DIR
 XDG_SESSION_  XDG_SESSION_  XDG_SESSION  XMODIFIERS   _
 CLASS         DESKTOP       _TYPE

include_variables
 <empty>

os
 bitness
 codename
 distribution
 edition
 family
 hostname
 name
 version

user
 config_dir
 data_dir
 data_local_dir
 document_dir
 home_dir
 id
 name
 username

variables
 <empty>

```

You can also view the values that these contexts have by passing in a `show-values` option as demonstrated below:

```shell
comtrya contexts --show-values
```

At the end, `comtrya contexts` also prints the directory where manifests fetched from git repositories are cached, e.g. `~/.cache/comtrya/manifests/git` on Linux. Comtrya updates these clones on every run, see [remote manifests](./cli.md#basic-usage-on-remote-manifests).

## Status

The **status** command provides an overview of manifests.

| Manifest          | Count of Actions |
| :---------------- | :--------------- |
| kubectl.krew      | 3                |
| kubectl.kubesess  | 3                |
| kdash.kdash       | 1                |

