//! The list of actions. Each action's name in manifests, its aliases and its type are written
//! once, in [`actions!`]: the [`Actions`] enum, its serde names, its `Display` and
//! [`Actions::NAMES`] all come from there, so they can't drift apart

use super::binary::BinaryGitHub;
use super::command::run::RunCommand;
use super::directory::{DirectoryCopy, DirectoryCreate, DirectoryRemove};
use super::file::{
    chown::FileChown, copy::FileCopy, download::FileDownload, link::FileLink, remove::FileRemove,
    unarchive::FileUnarchive,
};
use super::git::GitClone;
use super::group::add::GroupAdd;
use super::macos::MacOSDefault;
use super::package::{PackageInstall, PackageRepository};
use super::user::{add::UserAdd, add_group::UserAddGroup};
use super::{Action, ConditionalVariantAction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines [`Actions`] from a table of `Variant(Type) = "name", aliases: ["alias", …];`.
/// Every variant gets its name and aliases as its doc comment, which the JSON schema shows
macro_rules! actions {
    ($(
        $variant:ident($action:ty) = $name:literal
            $(, aliases: [$first_alias:literal $(, $alias:literal)*])?;
    )+) => {
        /// Every action, by its name in manifests
        #[derive(JsonSchema, Clone, Debug, Serialize, Deserialize)]
        #[serde(tag = "action")]
        pub enum Actions {
            $(
                #[doc = concat!(
                    "`", $name, "`"
                    $(, ", or `", $first_alias, "`" $(, ", `", $alias, "`")*)?
                )]
                #[serde(rename = $name $(, alias = $first_alias $(, alias = $alias)*)?)]
                $variant(ConditionalVariantAction<$action>),
            )+
        }

        impl Actions {
            /// Every action as its name and aliases, in the order of the enum
            pub const NAMES: &'static [(&'static str, &'static [&'static str])] = &[
                $(($name, &[$($first_alias $(, $alias)*)?]),)+
            ];

            /// The action's name in manifests
            pub fn name(&self) -> &'static str {
                match self {
                    $(Actions::$variant(_) => $name,)+
                }
            }

            /// The action, whichever it is
            pub fn inner_ref(&self) -> &(dyn Action + 'static) {
                match self {
                    $(Actions::$variant(action) => action,)+
                }
            }
        }
    };
}

actions! {
    CommandRun(RunCommand) = "command.run", aliases: ["cmd.run"];
    DirectoryCopy(DirectoryCopy) = "directory.copy", aliases: ["dir.copy"];
    DirectoryCreate(DirectoryCreate) = "directory.create", aliases: ["dir.create"];
    FileCopy(FileCopy) = "file.copy";
    FileChown(FileChown) = "file.chown";
    FileDownload(FileDownload) = "file.download";
    FileLink(FileLink) = "file.link";
    FileRemove(FileRemove) = "file.remove";
    FileUnarchive(FileUnarchive) = "file.unarchive";
    DirectoryRemove(DirectoryRemove) = "directory.remove", aliases: ["dir.remove"];
    BinaryGitHub(BinaryGitHub) = "binary.github", aliases: ["binary.gh", "bin.github", "bin.gh"];
    GitClone(GitClone) = "git.clone";
    GroupAdd(GroupAdd) = "group.add";
    MacOSDefault(MacOSDefault) = "macos.default";
    PackageInstall(PackageInstall) = "package.install", aliases: ["package.installed"];
    PackageRepository(PackageRepository) = "package.repository", aliases: ["package.repo"];
    UserAdd(UserAdd) = "user.add";
    UserAddGroup(UserAddGroup) = "user.group";
}

impl std::fmt::Display for Actions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl std::ops::Deref for Actions {
    type Target = dyn Action;

    fn deref(&self) -> &Self::Target {
        self.inner_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::Actions;
    use std::path::Path;

    /// The smallest manifest entry of each action that parses, apart from its name
    fn parse(name: &str) -> Result<Actions, serde_yaml_ng::Error> {
        serde_yaml_ng::from_str(&format!("action: {name}\n"))
    }

    #[test]
    fn every_name_and_alias_is_the_action() {
        let unknown = parse("file.nope").unwrap_err().to_string();
        assert!(unknown.contains("unknown variant"), "{unknown}");

        for (name, aliases) in Actions::NAMES {
            for spelling in std::iter::once(name).chain(aliases.iter()) {
                // Required fields are missing, but the error must be about them, not the name
                if let Err(err) = parse(spelling) {
                    assert!(
                        !err.to_string().contains("unknown variant"),
                        "{spelling}: {err}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_action_is_in_the_user_docs() {
        let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/src");
        let docs: String = std::fs::read_dir(&docs)
            .unwrap()
            .filter_map(|entry| std::fs::read_to_string(entry.unwrap().path()).ok())
            .collect();

        for (name, _) in Actions::NAMES {
            assert!(docs.contains(name), "{name} isn't documented in docs/src");
        }
    }
}
