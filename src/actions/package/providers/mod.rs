use self::homebrew::Homebrew;
use crate::actions::ActionError;
use serde::{Deserialize, Serialize};

mod homebrew;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PackageProviders {
    #[serde(alias = "homebrew", alias = "brew")]
    Homebrew,
}

impl PackageProviders {
    pub fn get_provider(self) -> Box<dyn PackageProvider> {
        match self {
            PackageProviders::Homebrew => Box::new(Homebrew {}),
        }
    }
}

impl Default for PackageProviders {
    fn default() -> Self {
        let info = os_info::get();

        match info.os_type() {
            // Debian / Ubuntu Variants
            // os_info::Type::Debian => PackageProviders::Apt,
            // os_info::Type::Mint => PackageProviders::Apt,
            // os_info::Type::Pop => PackageProviders::Apt,
            // os_info::Type::Ubuntu => PackageProviders::Apt,

            os_info::Type::Macos => PackageProviders::Homebrew,

            _ => panic!("Sorry, but we don't have a default provider for {} OS. Please be explicit when requesting a package installation with `provider: XYZ`.", info.os_type()),
        }
    }
}

pub trait PackageProvider {
    fn available(&self) -> bool;
    fn bootstrap(&self) -> Result<(), ActionError>;
    fn install(&self, packages: Vec<String>) -> Result<(), ActionError>;
}
