use std::{fs::File, path::PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::atoms::Atom;

use super::FileAtom;

pub struct Unarchive {
    pub origin: PathBuf,
    pub dest: PathBuf,
}

impl FileAtom for Unarchive {
    fn get_path(&self) -> &PathBuf {
        &self.origin
    }
}

impl Atom for Unarchive {
    // Determine if this atom needs to run
    fn plan(&self) -> bool {
        self.origin.exists()
    }

    // Apply new to old
    fn execute(&mut self) -> anyhow::Result<()> {
        let tar_gz = File::open(&self.origin)?;
        println!("hello");
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        archive.unpack(&self.dest)?;
        Ok(())
    }
}

impl std::fmt::Display for Unarchive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The archive {} to be decompressed to {}",
            self.origin.to_str().unwrap(),
            self.dest.to_str().unwrap(),
        )
    }
}
