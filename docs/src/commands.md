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
| version         | Print version information                    |
| contexts        | List available contexts                      |
| gen-completions | Auto generate completions                    |
| help            | Print out help information for using comtrya |

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
