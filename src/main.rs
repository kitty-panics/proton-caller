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
enum ProtonVersion {
    Mainline(u8, u8),
    Experimental,
    Custom,
}

impl Default for ProtonVersion {
    fn default() -> Self {
        ProtonVersion::Mainline(6, 3)
    }
}

impl ProtonVersion {
    pub fn new(major: u8, minor: u8) -> ProtonVersion {
        ProtonVersion::Mainline(major, minor)
    }

    pub fn from_custom(name: &Path) -> ProtonVersion {
        if let Some(n) = name.file_name() {
            if let Ok(n) = n.to_string_lossy().to_string().parse() {
                return n;
            }
        }

        ProtonVersion::Custom
    }
}

impl Display for ProtonVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtonVersion::Mainline(mj, mn) => write!(f, "{}.{}", mj, mn),
            ProtonVersion::Experimental => write!(f, "Experimental"),
            ProtonVersion::Custom => write!(f, "Custom"),
        }
    }
}

impl FromStr for ProtonVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_ascii_lowercase() == "experimental" {
            return Ok(ProtonVersion::Experimental);
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

    let config = Config::open()?;
    let mut parser = Jargon::from_vec(args);

    if parser.contains(["-h", "--help"]) {
        help();
    } else if parser.contains(["-v", "--version"]) {
        version();
    } else if parser.contains(["-i", "--index"]) {
        let common_index = CommonIndex::new(&config.common())?;
        println!("{}", common_index);
    } else {
        let args = Args {
            program: parser.result_arg(["-r", "--run"])?,
            version: parser.option_arg(["-p", "--proton"]).unwrap_or_default(),
            log: parser.contains(["-l", "--log"]),
            custom: parser.option_arg(["-c", "--custom"]),
            args: parser.finish(),
        };

        if args.custom.is_some() {
            custom_mode(config, args)?;
        } else {
            normal_mode(config, args)?;
        }
    }

    Ok(())
}

fn normal_mode(config: Config, args: Args) -> Result<(), Error> {
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

fn custom_mode(config: Config, args: Args) -> Result<(), Error> {
    if let Some(custom) = args.custom {
        let proton = Proton::new(
            ProtonVersion::from_custom(custom.as_path()),
            custom,
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
    } else {
        err!("failed to get custom path")
    }
}

#[derive(Debug)]
struct CommonIndex {
    dir: PathBuf,
    map: BTreeMap<ProtonVersion, PathBuf>,
}

impl Display for CommonIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut str: String = format!(
            "Indexed Directory: {}\n\nIndexed Proton Versions:\n",
            self.dir.to_string_lossy()
        );

        for (version, path) in &self.map {
            str = format!("{}\nProton {} `{}`", str, version, path.to_string_lossy());
        }

        write!(f, "{}", str)
    }
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

    pub fn run(mut self) -> Result<ExitStatus, Error> {
        use std::io::ErrorKind;

        let name = self.compat.to_string_lossy().to_string();
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

static HELP: &str = "\
Usage: proton-call [OPTIONS]... EXE [EXTRA]...

Options:
    -c, --custom [PATH]     Path to a directory containing Proton to use
    -h, --help              View this help message
    -i, --index             View an index of installed Proton versions
    -l, --log               Pass PROTON_LOG variable to Proton
    -p, --proton [VERSION]  Use Proton VERSION from `common`
    -r, --run EXE           Run EXE in proton
    -v, --verbose           Run in verbose mode
    -V, --version           View version information

Config:
    The config file should be located at '$XDG_CONFIG_HOME/proton.conf' or '$HOME/.config/proton.conf'
    The config requires two values.
    Data: a location to any directory to contain Proton's runtime files.
    Steam: the directory to where steam is installed (the one which contains the steamapps directory).
    Common: the directory to where your proton versions are stored, usually Steam's steamapps/common directory.
    Example:
        data = \"/home/avery/Documents/Proton/env/\"
        steam = \"/home/avery/.steam/steam/\"
        common = \"/home/avery/.steam/steam/steamapps/common/\"
";

fn help() {
    println!("{}", HELP);
}

fn version() {
    println!(
        "Proton Caller (proton-call) {} Copyright (C) 2021 {}",
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    );
}
