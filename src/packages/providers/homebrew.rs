use crate::packages::Package;
use reqwest::blocking::Client;
use std::{
    fs::File,
    process::{Command, Stdio},
};
use which::which;

pub struct Homebrew {}

impl Homebrew {
    fn supported(&self) -> bool {
        true
    }

    pub fn init(&self) -> Result<bool, super::PackageProviderError> {
        match which("brew") {
            Ok(_) => return Ok(false),
            Err(_) => (),
        };

        let client = Client::new();
        match client
            .get("https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh")
            .send()
        {
            Ok(mut res) => {
                let mut file = File::create("/tmp/brew-install.sh").unwrap();
                ::std::io::copy(&mut res, &mut file).unwrap();
            }
            Err(e) => {
                return Err(super::PackageProviderError);
            }
        };

        // Homebrew can only be used on Linux and macOS, so we can assume
        // we have access to bash ... right? 😅
        let mut installer = Command::new("bash")
            .args(&["/tmp/brew-install.sh"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .unwrap();

        println!("Brew install {:?}", String::from_utf8(installer.stdout));

        Ok(true)
    }

    fn add_repository(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }

    pub fn install(&self, package: &Package) -> Result<bool, super::PackageProviderError> {
        Command::new("brew")
            .arg(format!("install {}", package.list.join(" ")))
            .output()
            .unwrap();

        Ok(true)
    }

    fn upgrade(&self) -> Result<bool, super::PackageProviderError> {
        todo!()
    }
}
