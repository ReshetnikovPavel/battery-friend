use std::{collections::HashMap, fs, path::PathBuf};

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_poll_period")]
    pub poll_period: String,
    pub messages: HashMap<String, Message>,
}

fn default_poll_period() -> String {
    "2m".to_owned()
}

#[derive(serde::Deserialize, Debug)]
pub struct Message {
    #[serde(default = "default_status")]
    pub status: String,
    pub from: i64,
    pub to: i64,
    pub body: Option<String>,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub urgency: Option<String>,
}

fn default_status() -> String {
    "Discharging".to_owned()
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Something wrong with config directory")
        .join("battery-friend/config.toml")
}

pub fn load(path: &PathBuf) -> Config {
    println!("reloading config");
    toml::from_str(&fs::read_to_string(path).expect("Problem reading config"))
        .expect("Problem parsing config")
}
