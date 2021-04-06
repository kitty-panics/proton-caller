#![warn(clippy::all, clippy::pedantic)]
mod config;
mod proton;

use config::Config;
use proton::Proton;

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let args_count: usize = args.len();
    let program: &str = args[0].split('/').last().unwrap_or(&args[0]);

    if args.len() == 1 {
        println!("proton-call: missing arguments");
        println!("Try 'proton-call --help' for more information");
        return;
    }

    let custom = match args[1].as_str() {
        "--help" | "-h" => {
            help();
            return;
        }
        "--setup" | "-s" => {
            setup();
            return;
        }
        "--version" | "-v" => {
            pc_version();
            return;
        }

        "--custom" | "-c" => true,

        _ => false,
    };

    let config = match Config::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("proton-call: error: {}", e);
            exit(78)
        }
    };

    let proton = match Proton::new(config, custom, &args, args_count) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: error: {}", program, e);
            exit(70);
        }
    };

    if let Err(e) = proton.execute() {
        eprintln!("proton-call: error: {}", e);
    }
}

// messaging
fn help() {
    println!("Usage: proton-call VERSION PROGRAM");
    println!("   or: basename OPTION PATH PROGRAM");
    println!("Execute PROGRAM with Proton VERSION");
    println!("If specified, run proton PATH\n");
    println!("  -c, --custom PATH       use proton from PATH");
    println!("  -h, --help              display this help message");
    println!("  -s, --setup             display setup information");
    println!("  -v, --version           display version information");
}

fn pc_version() {
    println!("  proton-caller 2.2.4 Copyright (C) 2021  Avery Murray");
    println!("This program comes with ABSOLUTELY NO WARRANTY.");
    println!("This is free software, and you are welcome to redistribute it");
    println!("under certain conditions.\n")
}

fn setup() {
    println!("Configuration of proton-call requires a config file located at");
    println!("`~/.config/proton.conf` which is formatted like:\n");
    println!("  data = \"\"");
    println!("  common = \"\"\n");
    println!("`data` is used to give proton a directory used for compatibility data.\n");
    println!("`common` is a directory pointing to steam's common directory, where Proton");
    println!("and games are installed");
}
