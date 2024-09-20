# Commands

Comtrya offer several commands for use. They can be listed in the terminal by running comtrya with the help command.

```
comtrya help
```

The primary command of use will be the apply command, which will apply the actions of provided manifests to the system.

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

The apply command will execute and run the manifests. There are a few ways to do this. First, it to point to a directory
of manifests and have comtrya execute them all.

```shell
comtrya -d ./manifests apply
```

As shown, this is achieved with the `-d` option, which tells comtrya the directory that house the manifests to be
executed.

You can also specify the specific manifest(s).

```shell
comtrya apply -m one,two,three
```

The `-m` option is used to tell comtrya the specific manifests to run. Note that the name of the manifest (i.e. one.yaml)
is only the name of the manifest and does not contain any pathing information. So, `/manifests/one` is not a valid input.
So it is expected to be located in the directory of the manifests you are specifying to run.

Suppose you have a directory `manifests/` that contains the manifests `one.yaml` and `two.yaml`. You want to *only*
execute `one.yaml`. There are three ways to achieve this.

```shell
cd manifests/
comtrya apply -m one
```

Or

```shell
comtrya -d manifests/one.yaml apply
```

Or, the third and final way is a combination of the two.

```shell
comtrya -d manifests/ apply -m one
```

## Contexts

The contexts command is useful to see what comtrya knows about. This can be environment variables, included variables, information about the OS, user information and other variables. Below is an exmaple of the output.

```
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

You can also view the values that these contexts have by passing in a `show-values` option as demonstrated below.

```
comtrya contexts --show-values
```

## Status

Provides an overview of manifests.

```
+-------------------+------------------+
| Manifest          | Count of Actions |
+======================================+
| kubectl.krew      | 3                |
|-------------------+------------------|
| kubectl.kubesess  | 3                |
|-------------------+------------------|
| kdash.kdash       | 1                |
+-------------------+------------------+
```
