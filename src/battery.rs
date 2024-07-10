use std::{fs, str::FromStr};

static BATTERY_STATUS_FILEPATH: &str = "/sys/class/power_supply/BAT0/status";
static BATTERY_PERCENTAGE_FILEPATH: &str = "/sys/class/power_supply/BAT0/capacity";

pub fn status() -> Status {
    fs::read_to_string(BATTERY_STATUS_FILEPATH)
        .expect("Should have been able to read the file")
        .trim()
        .parse()
        .expect("There are not enough values in my enum")
}

pub fn percentage() -> i64 {
    fs::read_to_string(BATTERY_PERCENTAGE_FILEPATH)
        .expect("Should have been able to read the file")
        .trim()
        .parse()
        .expect("Should have been a number")
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
            _ => Err("Battery status is not written correctly".to_owned()),
        }
    }
}
