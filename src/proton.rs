use crate::config::Config;
use std::{
    io::{Error, ErrorKind},
    path::Path,
};

const PROTON_LATEST: &str = "5.13";

pub fn errorhere(ek: ErrorKind, info: &str) -> Result<Proton, Error> {
    Err(Error::new(ek, info))
}

pub struct Proton {
    proton: String,
    program: String,
    arguments: Vec<String>,
    config: Config,
}

impl Proton {
    /// Massive function to initiate the Proton struct.
    pub fn new(
        config: Config,
        custom: bool,
        args: &[String],
        args_count: usize,
    ) -> Result<Self, Error> {
        // Run custom mode instead.
        if custom {
            return Self::new_custom(config, args, args_count);
        }

        // check if arguments are valid
        if args[1].contains('-') {
            errorhere(ErrorKind::Other, &format!("invalid argument: '{}'", args[1]))?;
        } else if args_count < 2 {
            errorhere(ErrorKind::Other, "not enough arguments")?;
        }

        // load proton path
        let (path, def) = Self::locate_proton(&args[1], &config.common, 2)?;
        let program: String = args[def].split('/').last().ok_or_else(|| Error::new(ErrorKind::Other, "failed to split path"))?.to_string();
        let start: usize = def + 1;
        println!("Program:  {}", program);

        // create vector of arguments to pass to proton
        let a: Vec<String> = Self::arguments(start, args_count, &args, &program);

        // create struct
        let proton: Self = Self {
            proton: path,
            program,
            arguments: a,
            config,
        };

        // check for proton and program executables
        proton.check()?;

        Ok(proton)
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

    /// Checks if selected Proton version and Program exist. Returns
    /// `Ok<()>` on success, `Err<Error>` on failure.
    fn check(&self) -> Result<(), Error> {
        if !Path::new(&self.proton).exists() {
            errorhere(
                ErrorKind::NotFound,
                &format!("{} not found", self.proton))?;
        }

        if !Path::new(&self.program).exists() {
            errorhere(ErrorKind::NotFound,
                &format!("{} not found", self.program)
            )?;
        }

        Ok(())
    }

    /// Searches `common` for any directory containing `version` and returns
    /// `Ok(String)` with the path, or `Err(())` if none are found.
    fn locate_proton(
        version: &str,
        common: &str,
        mut ret: usize,
    ) -> Result<(String, usize), Error> {
        let dir: std::fs::ReadDir = std::fs::read_dir(&common)?;

        for path in dir {
            let p = path?.path();
            let d = p
                .to_str()
                .ok_or_else(|| Error::new(ErrorKind::Other, "failed to search common"))?;
            if d.contains(version) {
                println!(
                    "Proton:   {}",
                    d.split('/')
                        .last()
                        .ok_or_else(|| Error::new(ErrorKind::Other, "failed to split path"))?
                );
                return Ok((format!("{}/proton", d), ret));
            }
        }

        if ret == 1 {
            errorhere(ErrorKind::NotFound, "Proton 5.13 not found")?;
        } else if version.parse::<f32>().is_err() && ret != 1 {
            ret = 1;
        } else {
            eprintln!("warning: Proton {} not found, defaulting to {}", version, PROTON_LATEST);
        }

        Self::locate_proton(PROTON_LATEST, common, ret)
    }

    /// Initiate custom mode, only called by `init()`
    pub fn new_custom(config: Config, args: &[String], args_count: usize) -> Result<Self, Error> {
        // Check for valid arguments.
        if args_count < 4 {
            return Err(Error::new(ErrorKind::Other, "not enough arguments"));
        }

        // Load given path.
        let path: String = format!("{}/proton", args[2].clone());

        // Create arguements vector.
        let a: Vec<String> = Self::arguments(4, args_count, &args, &args[3]);

        let proton: Self = Self {
            proton: path,
            program: args[3].clone(),
            arguments: a,
            config,
        };

        // Check Proton.
        proton.check()?;

        println!("Proton:   custom");
        println!(
            "Program:  {}",
            args[3].split('/').last().ok_or_else(|| Error::new(
                ErrorKind::Other,
                "failed to split program at proton.rs::init_custom"
            ))?
        );

        Ok(proton)
    }

    /// Executes proton,,, Finally.
    pub fn execute(self) -> Result<(), Error> {
        println!("\n________Proton________");

        let log = if self.config.log {
            '1'.to_string()
        } else {
            '0'.to_string()
        };

        let mut child = std::process::Command::new(self.proton)
            .args(self.arguments)
            .env("STEAM_COMPAT_DATA_PATH", self.config.data)
            .env("PROTON_LOG", log)
            .spawn()?;

        let ecode = child.wait()?;
        if !ecode.success() {
            errorhere(
                ErrorKind::BrokenPipe,
                "Proton exited with an error",
            )?;
        }

        println!("______________________\n");
        Ok(())
    }
}
