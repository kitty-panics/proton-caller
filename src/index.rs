use crate::{err, Error, Version};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::fs::DirEntry;
use std::path::{Path, PathBuf};

/// Index type to Index Proton versions in common
#[derive(Debug)]
pub struct Index {
    dir: PathBuf,
    map: BTreeMap<Version, PathBuf>,
}

impl Display for Index {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut str: String = format!(
            "Indexed Directory: {}\n\nIndexed {} Proton Versions:\n",
            self.dir.to_string_lossy(),
            self.len()
        );

        for (version, path) in &self.map {
            str = format!("{}\nProton {} `{}`", str, version, path.to_string_lossy());
        }

        write!(f, "{}", str)
    }
}

impl Index {
    /// Creates an index of Proton versions in given path
    ///
    /// # Errors
    ///
    /// Will fail if Indexing fails to read the directory
    pub fn new(index: &Path) -> Result<Index, Error> {
        let mut idx = Index {
            dir: index.to_path_buf(),
            map: BTreeMap::new(),
        };

        idx.index()?;

        Ok(idx)
    }

    #[must_use]
    /// Returns the number of Indexed Protons
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    /// Returns true if Index is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[must_use]
    /// Retrieves the path of the requested Proton version
    pub fn get(&self, version: Version) -> Option<PathBuf> {
        let path = self.map.get(&version)?;
        Some(path.clone())
    }

    /// Indexes Proton versions
    fn index(&mut self) -> Result<(), Error> {
        if let Ok(rd) = self.dir.read_dir() {
            for result_entry in rd {
                let entry: DirEntry = match result_entry {
                    Ok(e) => e,
                    Err(e) => return err!("'{}' when reading common", e),
                };

                let entry_path: PathBuf = entry.path();

                if entry_path.is_dir() {
                    let name: OsString = entry.file_name();
                    let name: String = name.to_string_lossy().to_string();
                    if let Some(version_str) = name.split(' ').last() {
                        if let Ok(version) = version_str.parse() {
                            self.map.insert(version, entry_path);
                        }
                    }
                }
            }
        } else {
            return err!("can not read common dir");
        }

        Ok(())
    }
}
