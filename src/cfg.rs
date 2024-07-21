use std::{
    collections::HashMap,
    fmt::Display,
    fs, io,
    path::PathBuf,
    sync::{Arc, PoisonError, RwLock, RwLockWriteGuard},
};

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_poll")]
    pub poll: String,
    pub messages: HashMap<String, Message>,
}

pub fn default_poll() -> String {
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

#[derive(Debug)]
pub enum LoadError {
    Read(io::Error),
    Parse(toml::de::Error),
}

impl std::error::Error for LoadError {}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = "Unable to load config";
        match self {
            LoadError::Read(e) => write!(f, "{}: {}", m, e),
            LoadError::Parse(e) => write!(f, "{}: {}", m, e),
        }
    }
}

pub fn load(path: &PathBuf) -> Result<Config, LoadError> {
    let contents = fs::read_to_string(path).map_err(|e| LoadError::Read(e))?;
    let config = toml::from_str(&contents).map_err(|e| LoadError::Parse(e))?;
    Ok(config)
}

#[derive(Debug)]
pub enum ReloadError<'a> {
    Load(LoadError),
    Poison(PoisonError<RwLockWriteGuard<'a, Config>>),
}

impl<'a> Display for ReloadError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = "Unable to reload config";
        match self {
            ReloadError::Load(e) => write!(f, "{}: {}", m, e),
            ReloadError::Poison(e) => write!(f, "{}: {}", m, e),
        }
    }
}

impl<'a> std::error::Error for ReloadError<'a> {}

pub fn reload<'a>(
    config_path: &PathBuf,
    config_rw_lock: &'a Arc<RwLock<Config>>,
) -> Result<(), ReloadError<'a>> {
    let config = load(&config_path).map_err(|e| ReloadError::Load(e))?;
    let mut write_lock = config_rw_lock.write().map_err(|e| ReloadError::Poison(e))?;
    *write_lock = config;
    Ok(())
}
