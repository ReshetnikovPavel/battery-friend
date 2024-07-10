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

pub fn default_config_path() -> Option<PathBuf> {
    Some(
        dirs::config_dir()?
            .join("battery-friend")
            .join("config.toml"),
    )
}

pub fn load(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file at {}: {}", path.display(), e))?;
    Ok(toml::from_str(&contents).map_err(|e| format!("Failed to parse toml config: {}", e))?)
}
