#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]

/*!
# Proton Caller

This is the library which defines the Proton Caller API.

View [bin/proton-call.rs](bin/proton-call.rs) for more information about the `proton-call` executable.
*/

use std::ffi::OsString;
use std::io::{Error, ErrorKind, Result};
use std::fmt::Formatter;

/// This is the latest version of Proton at the time of modification.
pub static PROTON_LATEST: &str = "6.3";

#[derive(Debug)]
/// Main struct for PC, manages and contains all needed information to properly execute Proton.
pub struct Proton {
    steam: String,
    proton: ProtonPath,
    executable: String,
    passed_args: Vec<OsString>,
    data: String,
    log: bool,
}

impl Proton {
    /// Creates a new Proton structure, with a Config and Args struct.
    pub fn new<T: ProtonConfig + ProtonArgs>(config: &T, args: &T) -> Proton {
        let common: String = config.get_common();

        let steam: String = config.get_steam();
        let data: String = config.get_data();
        let proton: ProtonPath = args.get_proton(&common);
        let executable: String = args.get_executable();
        let passed_args: Vec<OsString> = args.get_extra_args();
        let log: bool = args.get_log();

        Proton {
            steam,
            proton,
            executable,
            passed_args,
            data,
            log,
        }
    }

    /// Checks the paths to Proton and the Executable to make sure they exist.
    ///
    /// # Errors
    ///
    /// Will fail if either requested Proton or executable don't exist.
    pub fn check(&self) -> Result<()> {
        use std::path::Path;

        if !Path::new(&self.proton.path()).exists() {
            return Err(Error::new(ErrorKind::NotFound, format!("{} not found!", self.proton)));
        }

        if !Path::new(&self.executable).exists() {
            return Err(Error::new(ErrorKind::NotFound, format!("{} not found!", self.executable)));
        }

        Ok(())
    }

    /// Executes Proton using information in the struct, drops `self` at the end.
    ///
    /// # Errors
    ///
    /// * Will fail if spawning Proton fails, view Proton's output for information about fixing this.
    /// * Will fail if waiting for Proton child fails. Nothing can be done for this.
    pub fn run(self) -> Result<()> {
        use std::process::{Child, Command};

        dbg!(&self);
        dbg!(self.proton.name());

        let log = if self.log { "1" } else { "0" };

        println!("\n________Proton________");

        let mut child: Child = Command::new(self.proton.path())
            .arg("run")
            .arg(self.executable)
            .args(self.passed_args)
            .env("PROTON_LOG", log)
            .env("STEAM_COMPAT_CLIENT_INSTALL_PATH", self.steam)
            .env("STEAM_COMPAT_DATA_PATH", self.data)
            .spawn()?;

        let exitcode = child.wait()?;
        println!("______________________\n");

        if !exitcode.success() {
            return Err(Error::new(ErrorKind::Other, "Proton exited with error"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ProtonPath {
    Custom {
        path: String,
    },

    Steam {
        version: String,
        name: String,
        path: String,
    }
}

impl ProtonPath {
    pub fn path(&self) -> String {
        match self {
            Self::Custom { path } => path.clone(),
            Self::Steam { path, .. } => path.clone(),
        }
    }

    pub fn version(&self) -> Option<String> {
        match self {
            Self::Custom { .. } => None,
            Self::Steam { version, .. } => Some(version.clone()),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Custom { path } => {
                let ret: Vec<&str> = path.split('/').collect();
                for s in ret {
                    if s == "proton" {
                        continue;
                    } else if s.starts_with("Proton") {
                        return s.to_string()
                    }
                }
                "custom".to_string()
            },
            Self::Steam { name, .. } => name.clone()
        }
    }
}

impl std::fmt::Display for ProtonPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",
            self.name()
        )
    }
}

/// Trait used in the Proton struct to get information from the Config.
pub trait ProtonConfig {
    /// Return `steam` directory where Steam is installed.
    fn get_steam(&self) -> String;

    /// Return `common` directory where Proton versions are installed to.
    fn get_common(&self) -> String;

    /// Return `data` directory use during Proton's runtime.
    fn get_data(&self) -> String;
}

/// Trait used in the Proton struct to get argument information from Command line.
pub trait ProtonArgs {
    /// Return full path to the requested proton executable.
    fn get_proton(&self, common: &str) -> ProtonPath;

    /// Return path to the Windows executable to run.
    fn get_executable(&self) -> String;

    /// Return any extra arguments to pass to the Windows executable, or empty vector if none.
    fn get_extra_args(&self) -> Vec<OsString>;

    /// Return value to use for the `PROTON_LOG` environment variable.
    fn get_log(&self) -> bool;
}
