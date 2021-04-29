mod file;

//
// Actions aren't really too smart. They're primarily compositions of base actions,
// lets think of them as Atoms. Creating a file, changing the attributions, permissions,
// Making an HTTP call, running a command, script, etc ... POSIX?
//
// So instead of allowing each of the contributors that brings a new Action to manually
// handle all these common Atoms, we should provide the bedrock / substrate.
//
// Should we rename Actions to Molecules?
//
// The current changeset proposal would be one or more Atoms.
// Actions would emit Atoms that can be played or reverted.
// This will also make testing easier, as we can verify that the Actions emit
// the correct set of atoms.
//

pub trait Atom {
    // Determine if this atom needs to run
    fn plan(&self) -> bool;

    // Apply new to old
    fn execute(&self) -> anyhow::Result<()>;
}

// pub struct FileSetContents {
//     file: FileAtom,
//     contents: String,
// }

// pub struct FileAppendContents {
//     file: FileAtom,
//     contents: String,
// }

// pub struct FileOwner {
//     file: FileAtom,
//     owner: String,
// }

// pub struct FileGroup {
//     file: FileAtom,
//     group: String,
// }

// pub struct FilePermissions {
//     file: FileAtom,
//     chmod: u32,
// }
