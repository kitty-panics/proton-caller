#[derive(serde_derive::Deserialize)]
pub struct Config {
    pub data: String,
    pub common: String,
    pub log: bool,
}

impl Config {
    pub fn new() -> Result<Config, &'static str> {
        let config: Config;
        let file: String;

        if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
            file = format!("{}/proton.conf", val);
        } else {
            file = format!("{}/.config/proton.conf", std::env::var("HOME").unwrap());
        }

        if !std::path::Path::new(&file).exists() {
            return Err("error: proton.conf does not exist");
        }

        let conf: String;

        match std::fs::read_to_string(file) {
            Ok(s) => conf = s,
            Err(_) => return Err("error: failed to read config"),
        }

        match toml::from_str(&conf) {
            Ok(o) => config = o,
            Err(_) => return Err("error: failed to read config"),
        }

        Ok(config)
    }
}
