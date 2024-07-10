use clap::Parser;
use notify_rust::{Notification, Urgency};
use std::{collections::HashMap, fs, path::PathBuf, process, str::FromStr, thread};

fn main() {
    let args = Args::parse();
    if !args.config.exists() {
        eprintln!(
            "Problem parsing arguments: config path `{}` does not exist",
            args.config.display()
        );
        process::exit(1);
    }

    let config: Config =
        toml::from_str(&fs::read_to_string(&args.config).expect("Problem reading config"))
            .expect("Problem parsing config");
    let duration = parse_duration::parse(&config.poll_period).expect("Wrong duration");

    let mut id = None;
    loop {
        let percent = percentage();
        for message in filter_messages(&config.messages, percent, status()) {
            let mut notification = build_notification(message, percent);
            if let Some(id) = id {
                notification.id(id);
            }
            let handle = notification.show().expect("Problem showing notification");
            id = Some(handle.id());
        }
        thread::sleep(duration)
    }
}

fn build_notification(message: &Message, percent: i64) -> Notification {
    let mut notification = Notification::new();
    if let Some(body) = &message.body {
        notification.body(&format(body, percent));
    }
    if let Some(summary) = &message.summary {
        notification.summary(&format(summary, percent));
    }
    if let Some(icon) = &message.icon {
        notification.icon(icon);
    }
    if let Some(urgency) = &message.urgency {
        let urgency = parse_urgency(urgency).expect("Problem parsing urgency");
        notification.urgency(urgency);
    }
    notification
}

fn format(string: &str, percent: i64) -> String {
    string.replace("{percent}", &percent.to_string())
}

fn parse_urgency(urgency: &str) -> Result<Urgency, &str> {
    match urgency {
        "low" | "Low" => Ok(Urgency::Low),
        "normal" | "Normal" => Ok(Urgency::Normal),
        "critical" | "Critical" => Ok(Urgency::Critical),
        _ => Err("Urgency is not written correctly"),
    }
}

fn filter_messages(
    messages: &HashMap<String, Message>,
    battery_percent: i64,
    status: BatteryStatus,
) -> Vec<&Message> {
    messages
        .iter()
        .filter(|(_, m)| m.status.parse::<BatteryStatus>().unwrap() == status)
        .filter(|(_, m)| m.from <= battery_percent && battery_percent <= m.to)
        .map(|(_, m)| m)
        .collect()
}

#[derive(serde::Deserialize, Debug)]
struct Config {
    #[serde(default = "default_poll_period")]
    poll_period: String,
    messages: HashMap<String, Message>,
}

fn default_poll_period() -> String {
    "2m".to_owned()
}

#[derive(serde::Deserialize, Debug)]
struct Message {
    #[serde(default = "default_status")]
    status: String,
    from: i64,
    to: i64,
    body: Option<String>,
    summary: Option<String>,
    icon: Option<String>,
    urgency: Option<String>,
}

fn default_status() -> String {
    "Discharging".to_owned()
}

fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Something wrong with config directory")
        .join("battery-friend/config.toml")
}

#[derive(Parser, Debug)]
#[command[version, about, long_about = None]]
struct Args {
    #[arg(short, long, default_value = default_config_path().into_os_string())]
    config: PathBuf,
}

static BATTERY_PERCENTAGE_FILEPATH: &str = "/sys/class/power_supply/BAT0/capacity";

fn percentage() -> i64 {
    fs::read_to_string(BATTERY_PERCENTAGE_FILEPATH)
        .expect("Should have been able to read the file")
        .trim()
        .parse()
        .expect("Should have been a number")
}

#[derive(PartialEq)]
enum BatteryStatus {
    Charging,
    NotCharging,
    Discharging,
}

impl FromStr for BatteryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "charging" | "Charging" => Ok(BatteryStatus::Charging),
            "not charging" | "Not charging" => Ok(BatteryStatus::NotCharging),
            "discharging" | "Discharging" => Ok(BatteryStatus::Discharging),
            _ => Err("Battery status is not written correctly".to_owned()),
        }
    }
}

static BATTERY_STATUS_FILEPATH: &str = "/sys/class/power_supply/BAT0/status";
fn status() -> BatteryStatus {
    fs::read_to_string(BATTERY_STATUS_FILEPATH)
        .expect("Should have been able to read the file")
        .trim()
        .parse()
        .expect("There are not enough values in my enum")
}
