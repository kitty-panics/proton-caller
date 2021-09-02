#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

/*!
# Proton Call

Run any Windows program through [Valve's Proton](https://github.com/ValveSoftware/Proton).

## Usage:

Defaults to the latest version of Proton.
`proton-call -r foo.exe`

Defaults to the latest version of Proton, all extra arguments passed to the executable.
`proton-call -r foo.exe --flags --for program`

Uses specified version of Proton, any extra arguments will be passed to the executable.
`proton-call -p 5.13 -r foo.exe`

Uses custom version of Proton, give the past to directory, not the Proton executable itself.
`proton-call -c '/path/to/Proton version' -r foo.exe`
*/

use proton_call::{Proton, ProtonArgs, ProtonConfig, PROTON_LATEST};
use std::ffi::OsString;
use std::fmt::Formatter;
use std::io::{Error, ErrorKind, Read};

type Result<T> = std::result::Result<T, ProtonCallerError>;

struct Caller {
    data: String,
    steam: String,
    common: Option<String>,
    proton: Option<String>,
    custom: Option<String>,
    program: String,
    log: bool,
    extra: Vec<OsString>,
}

impl Caller {
    pub fn new() -> Result<Caller> {
        use pico_args::Arguments;
        use std::collections::HashMap;
        use std::process::exit;

        let mut parser: Arguments = Arguments::from_env();

        if parser.contains(["-h", "--help"]) {
            println!("{}", HELP);
            exit(0);
        } else if parser.contains(["-V", "--version"]) {
            version();
            exit(0);
        }

        let cfg_path: String = Caller::locate_config()?;
        let config_dat: String = Caller::read_config(cfg_path)?;
        let config: HashMap<String, String> = toml::from_str(&config_dat)?;

        let common: Option<String> = if config.contains_key("common") {
            Some(config["common"].clone())
        } else {
            None
        };

        Ok(Caller {
            data: config["data"].clone(),
            steam: config["steam"].clone(),
            common,
            proton: parser.opt_value_from_str(["-p", "--proton"])?,
            custom: parser.opt_value_from_str(["-c", "--custom"])?,
            program: parser.value_from_str(["-r", "--run"])?,
            log: parser.contains(["-l", "--log"]),
            extra: parser.finish(),
        })
    }

    fn locate_config() -> Result<String> {
        use std::env::var;

        if let Ok(val) = var("XDG_CONFIG_HOME") {
            Ok(format!("{}/proton.conf", val))
        } else if let Ok(val) = var("HOME") {
            Ok(format!("{}/.config/proton.conf", val))
        } else {
            Err(ProtonCallerError::new("Failed to read environment!"))
        }
    }

    fn read_config(path: String) -> Result<String> {
        use std::fs::File;

        let mut file: File = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Err(ProtonCallerError::new("cannot open config file"));
                }
                return Err(ProtonCallerError::new(e.to_string()));
            }
        };

        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf)?;

        let config_dat: String = match String::from_utf8(buf) {
            Ok(s) => s,
            Err(e) => return Err(ProtonCallerError(e.to_string())),
        };

        Ok(config_dat)
    }
}

impl ProtonConfig for Caller {
    fn get_steam(&self) -> String {
        self.steam.clone()
    }

    fn get_common(&self) -> String {
        if self.common.is_none() {
            format!("{}/steamapps/common/", self.steam.clone())
        } else {
            self.common.clone().unwrap()
        }
    }

    fn get_data(&self) -> String {
        self.data.clone()
    }
}

impl ProtonArgs for Caller {
    fn get_proton(&self, common: &str) -> String {
        if let Some(path) = &self.custom {
            format!("{}/proton", path)
        } else {
            let proton: String = self
                .proton
                .clone()
                .unwrap_or_else(|| PROTON_LATEST.to_string());
            format!("{}/Proton {}/proton", common, proton)
        }
    }

    fn get_executable(&self) -> String {
        self.program.clone()
    }

    fn get_extra_args(&self) -> Vec<OsString> {
        self.extra.clone()
    }

    fn get_log(&self) -> bool {
        self.log
    }
}

fn main() {
    if let Err(e) = proton_caller() {
        eprintln!("{}error:{} {}", lliw::Fg::LightRed, lliw::Fg::Reset, e);
        std::process::exit(1);
    }
}

fn proton_caller() -> Result<()> {
    let caller: Caller = Caller::new()?;
    let proton: Proton = Proton::new(&caller, &caller);
    proton.check()?;
    proton.run()?;
    Ok(())
}

fn version() {
    println!(
        "Proton Caller (proton-call) {} Copyright (C) 2021 {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
}

static HELP: &str = "\
Usage: proton-call [OPTIONS]... EXE [EXTRA]...

Options:
    -c, --custom [PATH]     Path to a directory containing Proton to use
    -h, --help              View this help message
    -l, --log               Pass PROTON_LOG variable to Proton
    -p, --proton [VERSION]  Use Proton VERSION from `common`
    -r, --run EXE           Run EXE in proton
    -v, --verbose           Run in verbose mode
    -V, --version           View version information

Config:
    The config file should be located at '$XDG_CONFIG_HOME/proton.conf' or '$HOME/.config/proton.conf'

    The config requires two values.
    Data: a location to any directory to contain Proton's runtime files.
    Common: the directory where your proton versions are stored, usually Steams' common directory.

    Example:
        data = \"/home/avery/Documents/Proton/env/\"
        common = \"/home/avery/.steam/steam/steamapps/common/\"
";

/// Verbose println macro, checks `$v` to be true, if is, prints the text.
#[macro_export]
macro_rules! vprintln {
    ($v:expr, $fmt:literal, $($arg:expr),*) => {
        if $v { println!($fmt, $($arg),*) }
    };

    ($v:expr, $fmt:literal) => {
        if $v { println!($fmt) }
    }
}

struct ProtonCallerError(String);

impl ProtonCallerError {
    pub fn new<T: Into<String>>(s: T) -> Self {
        Self(s.into())
    }
}

impl From<pico_args::Error> for ProtonCallerError {
    fn from(e: pico_args::Error) -> Self {
        ProtonCallerError(e.to_string())
    }
}

impl From<ProtonCallerError> for Error {
    fn from(e: ProtonCallerError) -> Self {
        Error::new(ErrorKind::Other, e.to_string())
    }
}

impl std::fmt::Display for ProtonCallerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Error> for ProtonCallerError {
    fn from(e: Error) -> Self {
        Self(e.to_string())
    }
}

impl From<toml::de::Error> for ProtonCallerError {
    fn from(e: toml::de::Error) -> Self {
        Self(e.to_string())
    }
}
