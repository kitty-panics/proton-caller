use std::io::{Error, ErrorKind};

#[derive(serde_derive::Deserialize, Debug)]
pub struct Config {
    pub data: String,
    pub common: String,
    pub log: bool,
}

impl Config {
    pub fn new() -> Result<Self, Error> {
        let file;
        if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
            file = format!("{}/proton.conf", val);
        } else {
            file = match std::env::var("HOME") {
                Ok(h) => format!("{}/.config/proton.conf", h),
                Err(e) => return Err(Error::new(ErrorKind::Other, format!("{}", e))),
            };
        }

        if !std::path::Path::new(&file).exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                "proton.conf does not exist",
            ));
        }

        let conf: String = std::fs::read_to_string(file)?;

        let config = toml::from_str(&conf)?;

        Ok(config)
    }
}
