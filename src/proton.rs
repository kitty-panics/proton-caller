use std::io::{Error, ErrorKind};

use crate::config;

#[derive(Debug)]
pub(crate) struct Proton {
    proton: String,
    arguments: Vec<String>,
    conf: config::Config,
}

// const PROTON_LATEST: &str = "5.13";

impl Proton {
    /// Massive function to initiate the Proton struct.
    pub fn init(args: &[String], args_count: usize) -> Result<Self, Error> {
        // check if arguments are valid
        if if_arg(&args[1]) {
            return Err(Error::new(ErrorKind::Other, "invalid argument"));
        }
        if args_count < 2 {
            return Err(Error::new(ErrorKind::Other, "not enough arguments"));
        }

        // create needed variables
        let mut start: usize = 1;
        let version: String = args[1].to_string();
        let program: String;

        // load in config
        let config = config::Config::new()?;

        // load proton path
        let (path, def) = Self::locate_proton(&version, &config.common, 2)?;
        program = args[def].to_string();
        start += def;

        // check for proton and program executables
        if let (false, f) = Self::check([&path, &program].to_vec()) {
            return Err(Error::new(ErrorKind::NotFound,  format!("'{}' does not exist", f)));
        }

        // create vector of arguments to pass to proton
        let a: Vec<String> = Self::arguments(start, args_count, &args, &program);

        println!("Proton:   {}", path.split('/').last().unwrap());
        println!("Program:  {}", program.split('/').last().unwrap());

        Ok(Self {
            proton: format!("{}/proton", path),
            arguments: a,
            conf: config,
        })
    }

    /// might be a dumb way of creating arguements to pass into
    /// `Command::new()`
    fn arguments(start: usize, end: usize, args: &[String], program: &str) -> Vec<String> {
        let mut vector: Vec<String> = vec![std::string::String::new(); end - (start - 2)];

        vector[0] = std::string::String::from("run");
        vector[1] = program.to_string();

        for i in start..end {
            vector[i - (start - 2)] = args[i].to_string();
        }
        vector
    }

    /// check if all files in `file` vector exist,
    /// if either don't return `false`
    fn check(file: Vec<&String>) -> (bool, String) {
        for i in file {
            if !std::path::Path::new(i).exists() {
                return (false, i.to_string());
            }
        }
        (true, "".to_string())
    }

    /// Searches `common` for any directory containing `version` and returns
    /// `Ok(String)` with the path, or `Err(())` if none are found.
	fn locate_proton(version: &str, common: &str, mut ret: usize) -> Result<(String, usize), Error> {
		let dir: std::fs::ReadDir = std::fs::read_dir(&common)?;

        if version.parse::<f32>().is_err() {
            ret = 1;
        }

		for path in dir {
			let p = path.unwrap().path();
			let d = p.to_str().unwrap();
			if d.contains(version) {
				return Ok((d.to_string(), ret));
			}
		}

        eprintln!("proton-call: warning: {} not found, defaulting to 5.13", version);
		return Self::locate_proton("5.13", common, ret);
	}

    /// Initiate custom mode, only called by `init()`
    pub fn init_custom(args: &[String], args_count: usize) -> Result<Self, Error> {
        // check for valie arguments.
        if args_count < 4 {
            return Err(Error::new(ErrorKind::Other, "not enough arguments"));
        }

        // load path
        let path: String = args[2].to_string();

        if let (false, f) = Self::check([&path, &args[3]].to_vec()) {
            return Err(Error::new(ErrorKind::NotFound,  format!("'{}' does not exist", f)));
        }

        // create arguements vector.
        let a: Vec<String> = Self::arguments(4, args_count, &args, &args[3]);

        // load in config
        let config = config::Config::new()?;

        println!("Proton:   custom");
        println!("Program:  {}", args[3].split('/').last().unwrap());

        Ok(Self {
            proton: format!("{}/proton", path),
            arguments: a,
            conf: config,
        })
    }

    /// Executes proton,,, Finally.
    pub fn execute(self) -> Result<(), Error> {
        println!("\n________Proton________");

        let log = if self.conf.log {
            '1'.to_string()
        } else {
            '0'.to_string()
        };

        let mut child = std::process::Command::new(self.proton)
            .args(self.arguments)
            .env("STEAM_COMPAT_DATA_PATH", self.conf.data)
            .env("PROTON_LOG", log)
            .spawn()?;

        let ecode = child.wait()?;
        if !ecode.success() {
            return Err(Error::new(ErrorKind::BrokenPipe, "Proton exited with an error"));
        }

        println!("______________________\n");
        Ok(())
    }
}

/// check the args ig.
fn if_arg(the_arg: &str) -> bool {
    let arg: Vec<char> = the_arg.chars().collect();
    if arg[0] == '-' {
        return true;
    }
    false
}
