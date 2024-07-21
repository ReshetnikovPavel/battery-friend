use std::{fmt::Display, fs, io, str::FromStr};

static BATTERY_STATUS_FILEPATH: &str = "/sys/class/power_supply/BAT0/status";
static BATTERY_PERCENTAGE_FILEPATH: &str = "/sys/class/power_supply/BAT0/capacity";

#[derive(Debug)]
pub enum StatusError {
    Read(io::Error),
    Parse(ParseStatusError),
}

impl Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = "Unable to get battery status";
        match self {
            StatusError::Read(e) => write!(f, "{}: {}", m, e),
            StatusError::Parse(e) => write!(f, "{}: {}", m, e),
        }
    }
}

impl std::error::Error for StatusError {}

pub fn status() -> Result<Status, StatusError> {
    Ok(fs::read_to_string(BATTERY_STATUS_FILEPATH)
        .map_err(|e| StatusError::Read(e))?
        .trim()
        .parse()
        .map_err(|e| StatusError::Parse(e))?)
}

#[derive(Debug)]
pub enum PercentageError {
    Read(io::Error),
    Parse(std::num::ParseIntError),
}

impl Display for PercentageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = "Unable to get battery percentage";
        match self {
            PercentageError::Read(e) => write!(f, "{}: {}", m, e),
            PercentageError::Parse(e) => write!(f, "{}: {}", m, e),
        }
    }
}

pub fn percentage() -> Result<i64, PercentageError> {
    Ok(fs::read_to_string(BATTERY_PERCENTAGE_FILEPATH)
        .map_err(|e| PercentageError::Read(e))?
        .trim()
        .parse()
        .map_err(|e| PercentageError::Parse(e))?)
}

#[derive(PartialEq)]
pub enum Status {
    Charging,
    NotCharging,
    Discharging,
    Full,
}

#[derive(Debug)]
pub struct ParseStatusError {
    s: String,
}

impl Display for ParseStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error parsing battery status. Unknown status `{}`",
            self.s
        )
    }
}

impl FromStr for Status {
    type Err = ParseStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "charging" | "Charging" => Ok(Status::Charging),
            "not charging" | "Not charging" => Ok(Status::NotCharging),
            "discharging" | "Discharging" => Ok(Status::Discharging),
            "full" | "Full" => Ok(Status::Full),
            _ => Err(ParseStatusError { s: s.to_owned() }),
        }
    }
}
