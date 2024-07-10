mod battery;
mod cfg;

use clap::Parser;
use notify_rust::{Notification, Urgency};
use std::{collections::HashMap, fs, path::PathBuf, process, thread};

fn main() {
    let args = Args::parse();
    if !args.config.exists() {
        eprintln!(
            "Problem parsing arguments: config path `{}` does not exist",
            args.config.display()
        );
        process::exit(1);
    }

    let config: cfg::Config =
        toml::from_str(&fs::read_to_string(&args.config).expect("Problem reading config"))
            .expect("Problem parsing config");
    let duration = parse_duration::parse(&config.poll_period).expect("Wrong duration");

    let mut id = None;
    loop {
        let percent = battery::percentage();
        for message in filter_messages(&config.messages, percent, battery::status()) {
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

#[derive(Parser, Debug)]
#[command[version, about, long_about = None]]
struct Args {
    #[arg(short, long, default_value = cfg::default_config_path().into_os_string())]
    config: PathBuf,
}

fn build_notification(message: &cfg::Message, percent: i64) -> Notification {
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
    messages: &HashMap<String, cfg::Message>,
    battery_percent: i64,
    status: battery::Status,
) -> Vec<&cfg::Message> {
    messages
        .iter()
        .filter(|(_, m)| m.status.parse::<battery::Status>().unwrap() == status)
        .filter(|(_, m)| m.from <= battery_percent && battery_percent <= m.to)
        .map(|(_, m)| m)
        .collect()
}
