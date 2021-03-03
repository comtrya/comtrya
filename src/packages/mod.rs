use serde::{Deserialize, Serialize};
use std::process::Command;
use std::{io::Result, process::ExitStatus};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageStatus {
    Installed,
    Latest,
    Uninstalled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageProvider {
    Apt,
    Homebrew,
}

impl Default for PackageProvider {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            os_info::Type::Debian => PackageProvider::Apt,
            os_info::Type::Macos => PackageProvider::Homebrew,
            os_info::Type::Mint => PackageProvider::Apt,
            os_info::Type::Pop => PackageProvider::Apt,
            os_info::Type::Ubuntu => PackageProvider::Apt,
            _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Package {
    name: Option<String>,

    #[serde(default)]
    list: Vec<String>,

    #[serde(default)]
    provider: PackageProvider,

    ensure: PackageStatus,
}

pub trait PackageCommand {
    fn run_command(&self) -> (Result<ExitStatus>, Vec<u8>);
}

impl Package {
    pub fn name(&self) -> String {
        if self.name.is_some() {
            return self.name.clone().unwrap();
        }

        return self.list.join(" ");
    }

    pub fn get_package_list(&self) -> Vec<String> {
        if self.list.len() == 0 {
            return vec![self.name.clone().unwrap()];
        }

        return self.list.clone();
    }
}
impl PackageCommand for Package {
    fn run_command(&self) -> (Result<ExitStatus>, Vec<u8>) {
        let mut command = match self.provider {
            PackageProvider::Homebrew => Command::new("brew"),
            PackageProvider::Apt => Command::new("apt"),
        };

        let command = match self.provider {
            PackageProvider::Homebrew => command.arg("install").args(self.get_package_list()),
            PackageProvider::Apt => command
                .args(&["install", "-y"])
                .args(self.get_package_list()),
        };

        match command.status() {
            Ok(o) => (Ok(o), command.output().unwrap().stdout),
            Err(e) => (Err(e), command.output().unwrap().stderr),
        }
    }
}
