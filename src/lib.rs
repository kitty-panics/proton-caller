#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![forbid(unstable_features)]
#![forbid(missing_fragment_specifier)]
#![warn(clippy::all, clippy::pedantic)]

/*!
# Proton Caller API

This defines the internal API used in `proton-call` to run Proton
*/

mod config;
mod index;
mod version;

/// Contains the `Error` and `ErrorKind` types
pub mod error;

pub use config::Config;
use error::{Error, Kind};
pub use index::Index;
use std::borrow::Cow;
use std::fs::create_dir;
pub use version::Version;

use std::path::PathBuf;
use std::process::ExitStatus;

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
        let str: Cow<str> = self.path.to_string_lossy();
        let str: String = format!("{}/proton", str);
        self.path = PathBuf::from(str);
        self
    }

    fn create_p_dir(&mut self) -> Result<(), Error> {
        let name: Cow<str> = self.compat.to_string_lossy();
        let newdir: PathBuf = PathBuf::from(format!("{}/Proton {}", name, self.version));

        if !newdir.exists() {
            if let Err(e) = create_dir(&newdir) {
                throw!(Kind::ProtonDir, "failed to create Proton directory: {}", e);
            }
        }

        self.compat = newdir;

        pass!()
    }

    fn check_proton(&self) -> Result<(), Error> {
        if !self.path.exists() {
            throw!(Kind::ProtonMissing, "{}", self.version);
        }

        pass!()
    }

    fn check_program(&self) -> Result<(), Error> {
        if !self.program.exists() {
            throw!(Kind::ProgramMissing, "{}", self.program.to_string_lossy());
        }

        pass!()
    }

    /// Changes `compat` path to the version of Proton in use, creates the directory if doesn't already exist
    ///
    /// # Errors
    ///
    /// Will fail on:
    /// * Creating a Proton compat env directory fails
    /// * Executing Proton fails
    pub fn run(mut self) -> Result<ExitStatus, Error> {
        self.create_p_dir()?;
        self.check_proton()?;
        self.check_program()?;
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
            Err(e) => throw!(Kind::ProtonSpawn, "{}\nDebug:\n{:#?}", e, self),
        };

        let status: ExitStatus = match child.wait() {
            Ok(e) => e,
            Err(e) => throw!(Kind::ProtonWait, "'{}': {}", child.id(), e),
        };

        pass!(status)
    }
}
