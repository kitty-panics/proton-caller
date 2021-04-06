use std::io::{Error, ErrorKind};

#[derive(serde_derive::Deserialize)]
pub struct Config {
    pub data: String,
    pub common: String,
    pub log: bool,
}

impl Config {
    pub fn new() -> Result<Config, std::io::Error> {
    	let file;
        if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
            file = format!("{}/proton.conf", val);
        } else {
            file = format!("{}/.config/proton.conf", std::env::var("HOME").unwrap());
        }

        if !std::path::Path::new(&file).exists() {
            return Err(Error::new(ErrorKind::NotFound, "error: proton.conf does not exist"));
        }

		let conf: String = std::fs::read_to_string(file)?;
        let config = toml::from_str(&conf)?;

        Ok(config)
    }
}
