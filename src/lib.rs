#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

/*!
# Proton Caller API

This defines the internal API used in `proton-call` to run Proton
*/

mod config;
mod error;
mod index;
mod version;

pub use config::Config;
pub use error::Error;
pub use index::Index;
pub use version::Version;

use std::path::PathBuf;
use std::process::ExitStatus;

#[doc(hidden)]
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => { Err($crate::Error::new(format!($($arg)*))) }
}

/// Type to handle executing Proton
#[derive(Debug)]
pub struct Proton {
    version: Version,
    path: PathBuf,
    program: PathBuf,
    args: Vec<String>,
    log: bool,
    compat: PathBuf,
    steam: PathBuf,
}

impl Proton {
    #[must_use]
    /// Creates a new instance of `Proton`
    pub fn new(
        version: Version,
        path: PathBuf,
        program: PathBuf,
        args: Vec<String>,
        log: bool,
        compat: PathBuf,
        steam: PathBuf,
    ) -> Proton {
        Proton {
            version,
            path,
            program,
            args,
            log,
            compat,
            steam,
        }
        .update_path()
    }

    /// Appends the executable to the path
    fn update_path(mut self) -> Proton {
        let str = self.path.to_string_lossy();
        let str = format!("{}/proton", str);
        self.path = PathBuf::from(str);
        self
    }

    /// Changes `compat` path to the version of Proton in use, creates the directory if doesn't already exist
    ///
    /// # Errors
    ///
    /// Will fail on:
    /// * Creating a Proton compat env directory fails
    /// * Executing Proton fails
    pub fn run(mut self) -> Result<ExitStatus, Error> {
        use std::io::ErrorKind;

        let name = self.compat.to_string_lossy();
        let newdir = format!("{}/Proton {}", name, self.version);

        match std::fs::create_dir(&newdir) {
            Ok(_) => self.compat = PathBuf::from(newdir),
            Err(e) => {
                if e.kind() == ErrorKind::AlreadyExists {
                    self.compat = PathBuf::from(newdir);
                } else {
                    return err!("failed creating new directory: {}", e);
                }
            }
        }

        self.execute()
    }

    /// Executes Proton
    fn execute(self) -> Result<ExitStatus, Error> {
        use std::process::{Child, Command};

        println!(
            "Running Proton {} for {}",
            self.version,
            self.program.to_string_lossy()
        );

        let log: &str = if self.log { "1" } else { "0" };

        let mut child: Child = match Command::new(&self.path)
            .arg("run")
            .arg(&self.program)
            .args(&self.args)
            .env("PROTON_LOG", log)
            .env("STEAM_COMPAT_DATA_PATH", &self.compat)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", &self.steam)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => return err!("failed spawning child: {}\n{:#?}", e, self),
        };

        let status = match child.wait() {
            Ok(e) => e,
            Err(e) => return err!("failed waiting for child '{}': {}", child.id(), e),
        };

        Ok(status)
    }
}
