//! Where clones live in the cache: one directory per repository and branch

use super::location::GitConfig;

/// Directory where manifests fetched from git repositories are cloned to,
/// one subdirectory per repository and branch
pub fn git_cache_dir() -> Option<std::path::PathBuf> {
    dirs::cache_dir().map(|dir| dir.join("comtrya").join("manifests").join("git"))
}

/// The name of the clone's directory in the cache: one per repository and branch
pub(super) fn cache_name(config: &GitConfig) -> String {
    let repository = clean_git_url(&config.repository);
    match &config.branch {
        // Branch names may contain `/`, which would nest the directory
        Some(branch) => format!("{repository}#{}", branch.replace(['/', '\\', ':'], "-")),
        None => repository,
    }
}

/// The repository URL as a single path segment
fn clean_git_url(uri: &str) -> String {
    uri.to_string()
        .replace("https", "")
        .replace("http", "")
        .replace([':', '.', '/'], "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn cache_dir_is_inside_the_platform_cache_dir() {
        let cache_dir = git_cache_dir().unwrap();

        assert!(cache_dir.starts_with(dirs::cache_dir().unwrap()));
        assert!(cache_dir.ends_with(
            std::path::Path::new("comtrya")
                .join("manifests")
                .join("git")
        ));
    }

    #[test]
    fn each_branch_has_its_own_clone() {
        let name = |location: &str| cache_name(&GitConfig::parse(location));

        assert_eq!(
            "githubcomcomtryacomtrya",
            name("https://github.com/comtrya/comtrya")
        );
        assert_eq!(
            name("https://github.com/comtrya/comtrya#:examples"),
            name("https://github.com/comtrya/comtrya")
        );
        assert_ne!(
            name("https://github.com/comtrya/comtrya#feat/x"),
            name("https://github.com/comtrya/comtrya")
        );
        assert!(!name("https://github.com/comtrya/comtrya#feat/x").contains('/'));
    }

    #[test]
    fn clean_git_url_is_a_single_path_segment() {
        assert_eq!(
            "githubcomcomtryacomtrya",
            clean_git_url("https://github.com/comtrya/comtrya")
        );
    }
}
