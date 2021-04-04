use crate::config;
pub(crate) struct Proton {
    proton: String,
    arguments: Vec<String>,
    conf: config::Config,
}

const PROTON_LATEST: &str = "5.13";

impl Proton {
    /// Massive function to initiate the Proton struct.
    ///
    /// It starts to check if it is in custom mode, if it is, it returns just
    /// what the `init_custom()` returns.
    ///
    /// It then loads the config into the struct
    pub fn init(args: &[String], custom: bool) -> Result<Proton, &'static str> {
        // init custom mode instead.
        if custom {
            return Proton::init_custom(&args);
        }

        // check if arguments are valid
        if if_arg(&args[1]) {
            return Err("error: invalid argument");
        }
        let args_len: usize = args.len();
        if args_len < 2 {
            return Err("error: not enough arguments");
        }

        // create needed variables
        let mut start: usize = 3;
        let version: String = args[1].to_string();
        let program: String;

        // load in config
        let config = match config::Config::new() {
            Ok(val) => val,
            Err(e) => return Err(e),
        };

        // load proton path
        let path = match Proton::locate_proton(&version, &config.common) {
            Ok(f) => {
                program = args[2].to_string();
                f
            }
            Err(_) => {
                if let Ok(v) = Proton::locate_proton(PROTON_LATEST, &config.common) {
                    program = args[1].to_string();
                    start = 2;
                    v
                } else {
                    return Err("error: cannot locate proton");
                }
            }
        };

        // check for proton and program executables
        if !Proton::check([&path, &program].to_vec()) {
            return Err("error: invalid Proton or executable");
        }

        // create vector of arguments to pass to proton
        let a: Vec<String> = Proton::arguments(start, args_len, &args, &program);

        println!("Proton:   {}", path.split('/').last().unwrap());
        println!("Program:  {}", program.split('/').last().unwrap());

        Ok(Proton {
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
    fn check(file: Vec<&String>) -> bool {
        for i in file {
            if !std::path::Path::new(i).exists() {
                return false;
            }
        }
        true
    }

    /// Searches `common` for any directory containing `version` and returns
    /// `Ok(String)` with the path, or `Err(())` if none are found.
    fn locate_proton(version: &str, common: &str) -> Result<String, ()> {
        let dir: std::fs::ReadDir;
        match std::fs::read_dir(common) {
            Ok(val) => dir = val,
            Err(_) => return Err(()),
        }

        for path in dir {
            let p = path.unwrap().path();
            let d = p.to_str().unwrap();
            if d.contains(version) {
                return Ok(d.to_string());
            }
        }
        Err(())
    }

    /// Initiate custom mode, only called by `init()`
    fn init_custom(args: &[String]) -> Result<Proton, &'static str> {
        // check for valie arguments.
        let args_len: usize = args.len();
        if args_len < 4 {
            return Err("error: not enough arguments");
        }

        // load path
        let path: String = args[2].to_string();

        if !Proton::check([&path, &args[3]].to_vec()) {
            return Err("error: invalid Proton or executable");
        }

        // create arguements vector.
        let a: Vec<String> = Proton::arguments(4, args_len, &args, &args[3]);

        // load in config
        let config = match config::Config::new() {
            Ok(val) => val,
            Err(e) => return Err(e),
        };

        println!("Proton:   custom");
        println!("Program:  {}", args[3].split('/').last().unwrap());

        Ok(Proton {
            proton: format!("{}/proton", path),
            arguments: a,
            conf: config,
        })
    }

    /// Executes proton,,, Finally.
    pub fn execute(self) -> Result<(), &'static str> {
        let ecode: std::process::ExitStatus;
        let mut child: std::process::Child;
        println!("\n________Proton________");

        let log = if self.conf.log {
            '1'.to_string()
        } else {
            '0'.to_string()
        };

        match std::process::Command::new(self.proton)
            .args(self.arguments)
            .env("STEAM_COMPAT_DATA_PATH", self.conf.data)
            .env("PROTON_LOG", log)
            .spawn()
        {
            Ok(val) => child = val,
            Err(_) => return Err("error: failed to launch Proton"),
        }

        match child.wait() {
            Ok(val) => ecode = val,
            Err(_) => return Err("error: failed to wait for Proton"),
        }

        if !ecode.success() {
            return Err("error: Proton exited with an error");
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
