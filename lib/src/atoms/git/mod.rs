//! Cloning and updating repositories, with the git built into comtrya

mod clone;
pub(crate) use clone::clone_repository;
pub use clone::Clone;

mod update;
pub use update::Update;

/// Writes a [`gix::Url`] as its string, for atoms that serialize: gix's own serde support would
/// need another feature in gix and its crates
pub(crate) mod url_as_string {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    /// The URL as gix prints it
    pub fn serialize<S: Serializer>(url: &gix::Url, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&url.to_bstring().to_string())
    }

    /// The URL parsed by gix
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<gix::Url, D::Error> {
        let url = String::deserialize(deserializer)?;
        gix::Url::from_bytes(url.as_str().into()).map_err(D::Error::custom)
    }
}
