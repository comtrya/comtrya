//! `package.install` and `package.repository`: packages and repositories of the system's
//! package manager, the "provider"

mod install;
mod providers;
mod repository;

use crate::actions::Ignored;
pub(crate) use install::PackageInstall;
pub use providers::PackageProviders;
pub(crate) use repository::PackageRepository;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Installs packages with a package manager. Packages that are already installed are skipped
/// where the provider can tell
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename = "package.install")]
pub struct Package {
    /// The package to install. Wins over `list`
    name: Option<String>,

    /// Several packages to install, when `name` isn't set
    #[serde(default)]
    list: Vec<String>,

    /// The package manager to use. Without it, `package_provider` from `Comtrya.yaml` or the
    /// default of the OS is used
    #[serde(default)]
    provider: Option<PackageProviders>,

    /// Ignored with a warning. Add repositories with `package.repository`
    #[serde(default)]
    repository: Option<String>,

    /// Ignored with a warning. Add repository keys with `package.repository`
    #[serde(default)]
    key: Option<String>,

    /// Arguments for the install command, before the packages
    #[serde(default)]
    extra_args: Vec<String>,

    /// Install from a local file, like a `.pkg` or `.deb` file, given as `name`. Only `bsdpkg`
    /// and `aptitude` support it
    #[serde(default)]
    file: bool,
}

/// What `package.install` installs, as the providers take it
#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PackageVariant {
    /// The package to install. Wins over `list`
    name: Option<String>,

    /// Several packages to install, when `name` isn't set
    #[serde(default)]
    list: Vec<String>,

    /// The package manager to use. Without it, `package_provider` from `Comtrya.yaml` or the
    /// default of the OS is used
    #[serde(default)]
    provider: Option<PackageProviders>,

    /// Arguments for the install command, before the packages
    #[serde(default)]
    extra_args: Vec<String>,

    /// Install from a local file given as `name`
    #[serde(default)]
    file: bool,
}

impl Package {
    /// The fields of this action that are set but not used
    fn ignored_fields(&self) -> Vec<Ignored> {
        let instead = "add it with `package.repository`";
        [
            ("repository", self.repository.is_some()),
            ("key", self.key.is_some()),
        ]
        .into_iter()
        .filter(|(_, set)| *set)
        .map(|(field, _)| (field, instead))
        .collect()
    }
}

impl PackageVariant {
    /// `name`, or else `list`
    fn packages(&self) -> Vec<String> {
        self.name
            .as_ref()
            .map(|s| vec![s.clone()])
            .unwrap_or_else(|| self.list.clone())
    }
}

impl From<&Package> for PackageVariant {
    fn from(package: &Package) -> Self {
        PackageVariant {
            name: package.name.clone(),
            list: package.list.clone(),
            provider: package.provider.clone(),
            extra_args: package.extra_args.clone(),
            file: package.file,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repository::RepositoryKey;

    #[test]
    fn install_names_its_ignored_fields() {
        let package = Package {
            name: Some(String::from("curl")),
            repository: Some(String::from("ppa:curl/stable")),
            key: Some(String::from("https://example.com/key.asc")),
            ..Default::default()
        };

        let fields: Vec<&str> = package.ignored_fields().iter().map(|(f, _)| *f).collect();
        assert_eq!(vec!["repository", "key"], fields);
        assert!(Package::default().ignored_fields().is_empty());
    }

    #[test]
    fn repository_names_the_ignored_fields_of_its_key() {
        let repository = PackageRepository {
            name: String::from("https://example.com/apt stable main"),
            key: Some(RepositoryKey {
                url: String::from("https://example.com/key.asc"),
                key: Some(String::from("ABCD")),
                fingerprint: Some(String::from("0123")),
                ..Default::default()
            }),
            ..Default::default()
        };

        let fields: Vec<&str> = repository
            .ignored_fields()
            .iter()
            .map(|(f, _)| *f)
            .collect();
        assert_eq!(vec!["key.key", "key.fingerprint"], fields);
        assert!(PackageRepository::default().ignored_fields().is_empty());
    }
}
