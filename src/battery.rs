use std::{fs, str::FromStr};

static BATTERY_STATUS_FILEPATH: &str = "/sys/class/power_supply/BAT0/status";
static BATTERY_PERCENTAGE_FILEPATH: &str = "/sys/class/power_supply/BAT0/capacity";

pub fn status() -> Result<Status, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(BATTERY_STATUS_FILEPATH)
        .map_err(|e| {
            format!(
                "Failed to read battery status file at {}, {}",
                BATTERY_STATUS_FILEPATH, e
            )
        })?
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse battery status: {}", e))?)
}

pub fn percentage() -> Result<i64, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(BATTERY_PERCENTAGE_FILEPATH)
        .map_err(|e| {
            format!(
                "Failed to read battery percentage file at {}, {}",
                BATTERY_PERCENTAGE_FILEPATH, e
            )
        })?
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse battery percentage: {}", e))?)
}

#[derive(PartialEq)]
pub enum Status {
    Charging,
    NotCharging,
    Discharging,
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "charging" | "Charging" => Ok(Status::Charging),
            "not charging" | "Not charging" => Ok(Status::NotCharging),
            "discharging" | "Discharging" => Ok(Status::Discharging),
            _ => Err("Battery status is not written correctly. \
                Possible values are: `charging`, `Charging`, \
                `not charging`, `Not charging`, \
                `discharging`, `Discharging`"
                .to_owned()),
        }
    }
}
