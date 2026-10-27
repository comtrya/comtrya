//! The `os` context

use crate::values::Value;
use gethostname::gethostname;
use std::collections::BTreeMap;

/// The operating system, the `os` context
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OsFacts {
    /// The name of this machine
    pub hostname: String,
    /// `unix` or `windows`
    pub family: String,
    /// `linux`, `macos`, `windows`, …
    pub name: String,
    /// The OS as `os_info` names it, like `Debian` or `Mac OS`
    pub distribution: String,
    /// The release name, like `trixie`, where the OS has one
    pub codename: Option<String>,
    /// `32-bit` or `64-bit`
    pub bitness: String,
    /// The version of the OS
    pub version: String,
    /// The edition, like `Pro` on Windows, where the OS has one
    pub edition: Option<String>,
}

impl OsFacts {
    /// Looks at the running OS
    pub fn gather() -> Self {
        let osinfo = os_info::get();

        Self {
            hostname: gethostname().to_str().unwrap_or("unknown").to_string(),
            family: std::env::consts::FAMILY.to_string(),
            name: std::env::consts::OS.to_string(),
            distribution: osinfo.os_type().to_string(),
            codename: osinfo.codename().map(String::from),
            bitness: osinfo.bitness().to_string(),
            version: osinfo.version().to_string(),
            edition: osinfo.edition().map(String::from),
        }
    }

    /// The `os` context. Missing values are `unknown`
    pub fn to_context(&self) -> BTreeMap<String, Value> {
        let or_unknown = |value: &Option<String>| value.as_deref().unwrap_or("unknown").into();

        BTreeMap::from([
            (String::from("hostname"), self.hostname.clone().into()),
            (String::from("family"), self.family.clone().into()),
            (String::from("name"), self.name.clone().into()),
            (
                String::from("distribution"),
                self.distribution.clone().into(),
            ),
            (String::from("codename"), or_unknown(&self.codename)),
            (String::from("bitness"), self.bitness.clone().into()),
            (String::from("version"), self.version.clone().into()),
            (String::from("edition"), or_unknown(&self.edition)),
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn family_and_name_come_from_the_target() {
        let os = OsFacts::gather();

        #[cfg(target_os = "macos")]
        assert_eq!(("unix", "macos"), (os.family.as_str(), os.name.as_str()));
        #[cfg(target_os = "linux")]
        assert_eq!(("unix", "linux"), (os.family.as_str(), os.name.as_str()));
        #[cfg(target_os = "freebsd")]
        assert_eq!(("unix", "freebsd"), (os.family.as_str(), os.name.as_str()));
        #[cfg(windows)]
        assert_eq!(
            ("windows", "windows"),
            (os.family.as_str(), os.name.as_str())
        );
    }

    #[test]
    fn missing_values_are_unknown_in_the_context() {
        let context = OsFacts::default().to_context();

        assert_eq!("unknown", context["codename"].to_string());
        assert_eq!("unknown", context["edition"].to_string());
    }
}
