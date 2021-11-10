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

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => { Err($crate::Error::new(format!($($arg)*))) }
}

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::str::FromStr;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Config {
    data: PathBuf,
    steam: PathBuf,
    common: Option<PathBuf>,
}

impl Config {
    /// Opens and returns the user's config
    pub fn open() -> Result<Config, Error> {
        use std::fs::File;
        use std::io::Read;

        // Get default config location
        let loc: PathBuf = Config::config_location()?;

        // Open the config file
        let mut file: File = match File::open(&loc) {
            Ok(f) => f,
            Err(e) => return err!("'{}' when opening config", e),
        };

        // Read the config into memory
        let mut buffer: Vec<u8> = Vec::new();

        if let Err(e) = file.read_to_end(&mut buffer) {
            return err!("'{}' when reading config", e);
        }

        // Parse the config into the structure
        let slice = buffer.as_slice();

        let mut config: Config = match toml::from_slice(slice) {
            Ok(c) => c,
            Err(e) => return err!("'{}' when parsing config", e),
        };

        config.default_common();

        Ok(config)
    }

    /// Finds one of the two default config locations
    pub fn config_location() -> Result<PathBuf, Error> {
        use std::env::var;

        if let Ok(val) = var("XDG_CONFIG_HOME") {
            let path = format!("{}/proton.conf", val);
            Ok(PathBuf::from(path))
        } else if let Ok(val) = var("HOME") {
            let path = format!("{}/.config/proton.conf", val);
            Ok(PathBuf::from(path))
        } else {
            err!("cannot read required environment variables")
        }
    }

    // Sets a default common if not given by user
    fn default_common(&mut self) {
        if self.common.is_none() {
            let common = self._default_common();
            self.common = Some(common);
        }
    }

    fn _default_common(&self) -> PathBuf {
        let steam = self.steam.to_string_lossy().to_string();
        let common_str = format!("{}/steamapps/common/", steam);
        PathBuf::from(common_str)
    }

    pub fn common(&self) -> PathBuf {
        if let Some(common) = &self.common {
            common.clone()
        } else {
            self._default_common()
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct ProtonVersion {
    major: u8,
    minor: u8,
}

impl Default for ProtonVersion {
    fn default() -> Self {
        ProtonVersion { major: 6, minor: 3 }
    }
}

const EXPR_STR: &str = "Experimental";
const EXPR_VER: ProtonVersion = ProtonVersion {
    major: u8::MAX,
    minor: u8::MAX,
};

impl ProtonVersion {
    pub fn new(major: u8, minor: u8) -> ProtonVersion {
        ProtonVersion { major, minor }
    }
}

impl Display for ProtonVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for ProtonVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == EXPR_STR {
            return Ok(EXPR_VER);
        }

        match s.split('.').collect::<Vec<&str>>().as_slice() {
            [maj, min] => Ok(ProtonVersion::new(maj.parse()?, min.parse()?)),
            _ => err!("failed to parse '{}'", s),
        }
    }
}

#[derive(Debug)]
struct Args {
    program: PathBuf,
    version: ProtonVersion,
    log: bool,
    custom: Option<PathBuf>,
    args: Vec<String>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program: String = args[0].split('/').last().unwrap_or(&args[0]).to_string();
    if let Err(e) = proton_caller(args) {
        eprintln!("{}: error: {}", program, e);
    }
}

fn proton_caller(args: Vec<String>) -> Result<(), Error> {
    use jargon_args::Jargon;

    let mut parser = Jargon::from_vec(args);

    if parser.contains(["-h", "--help"]) {
        todo!("help");
    } else if parser.contains(["-v", "--version"]) {
        todo!("version");
    } else {
        let args = Args {
            program: parser.result_arg(["-r", "--run"])?,
            version: parser.option_arg(["-p", "--proton"]).unwrap_or_default(),
            log: parser.contains(["-l", "--log"]),
            custom: parser.option_arg(["-c", "--custom"]),
            args: parser.finish(),
        };

        if args.custom.is_some() {
            todo!("custom mode");
        } else {
            normal_mode(args)?;
        }
    }

    Ok(())
}

fn normal_mode(args: Args) -> Result<(), Error> {
    let config = Config::open()?;
    let common_index = CommonIndex::new(&config.common())?;
    let proton_path = match common_index.get(args.version) {
        Some(pp) => pp,
        None => return err!("Proton {} is not found", args.version),
    };

    let proton = Proton::new(
        args.version,
        proton_path,
        args.program,
        args.args,
        args.log,
        config.data,
        config.steam,
    );

    if proton.run()?.success() {
        Ok(())
    } else {
        err!("Proton exited with an error")
    }
}

#[derive(Debug)]
struct CommonIndex {
    dir: PathBuf,
    map: BTreeMap<ProtonVersion, PathBuf>,
}

impl CommonIndex {
    pub fn new(index: &Path) -> Result<CommonIndex, Error> {
        let mut idx = CommonIndex {
            dir: index.to_path_buf(),
            map: BTreeMap::new(),
        };
        idx.index()?;
        Ok(idx)
    }

    pub fn get(&self, version: ProtonVersion) -> Option<PathBuf> {
        let path = self.map.get(&version)?;
        Some(path.clone())
    }

    fn index(&mut self) -> Result<(), Error> {
        if let Ok(rd) = self.dir.read_dir() {
            for result_entry in rd {
                let entry = match result_entry {
                    Ok(e) => e,
                    Err(e) => return err!("'{}' when reading common", e),
                };

                let entry_path = entry.path();

                if entry_path.is_dir() {
                    let name = entry.file_name();
                    let name = name.to_string_lossy().to_string();
                    if name.starts_with("Proton ") {
                        if let Some(version_str) = name.split(' ').last() {
                            let version = version_str.parse()?;
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

#[derive(Debug)]
struct Proton {
    version: ProtonVersion,
    path: PathBuf,
    program: PathBuf,
    args: Vec<String>,
    log: bool,
    compat: PathBuf,
    steam: PathBuf,
}

impl Proton {
    pub fn new(
        version: ProtonVersion,
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

    fn update_path(mut self) -> Proton {
        let mut str = self.path.to_string_lossy().to_string();
        str = format!("{}/proton", str);
        self.path = PathBuf::from(str);
        self
    }

    fn run(self) -> Result<ExitStatus, Error> {
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

/*
use proton_call::{Proton, ProtonArgs, ProtonConfig, PROTON_LATEST, ProtonPath};
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
    fn get_proton(&self) -> ProtonPath {
        if let Some(path) = &self.custom {
            ProtonPath::Custom {
                path: path.to_string(),
            }
        } else {
            let version: String = self
                .proton
                .clone()
                .unwrap_or_else(|| PROTON_LATEST.to_string());

            let name: String = format!("Proton {}", version);
            let path: String = format!("{}/{}/proton", self.get_common(), name);

            ProtonPath::Steam {
                version,
                name,
                path
            }
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
*/

#[derive(Debug)]
struct Error(String);

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error {
    pub fn new(info: String) -> Error {
        Error(info)
    }
}

impl From<ParseIntError> for Error {
    fn from(pie: ParseIntError) -> Self {
        Self(pie.to_string())
    }
}

impl From<jargon_args::Error> for Error {
    fn from(jae: jargon_args::Error) -> Self {
        match jae {
            jargon_args::Error::MissingArg(e) => Error::new(format!("missing argument: '{}'", e)),
            jargon_args::Error::Other(e) => Error::new(e),
        }
    }
}
