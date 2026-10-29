//! Cloning and updating repositories, with the git built into comtrya

mod clone;
pub(crate) use clone::clone_repository;
pub use clone::Clone;

mod update;
pub use update::Update;
