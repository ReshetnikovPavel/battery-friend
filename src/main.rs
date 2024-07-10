mod battery;
mod cfg;

use clap::Parser;
use notify::{Error, Event, RecommendedWatcher, RecursiveMode, Watcher};
use notify_rust::{Notification, Urgency};
use std::{
    collections::HashMap,
    path::PathBuf,
    process,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

fn main() {
    let args = Args::parse();
    if !args.config.exists() {
        eprintln!(
            "Problem parsing arguments: config path `{}` does not exist",
            args.config.display()
        );
        process::exit(1);
    }
    let config_path = args.config;
    let cloned_config_path = config_path.clone();

    let config = cfg::load(&config_path).expect("Failed to load config");
    let config_rw_lock = Arc::new(RwLock::new(config));
    let cloned_config_rw_lock = config_rw_lock.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, Error>| {
            let event = result.unwrap();
            if event.kind.is_modify() {
                for _ in 0..10 {
                    match cfg::load(&cloned_config_path) {
                        Ok(config) => {
                            *cloned_config_rw_lock.write().unwrap() = config;
                            eprintln!("Config reloaded successfully");
                            return;
                        }
                        Err(_) => thread::sleep(Duration::from_millis(10)),
                    }
                }
                eprintln!("Failed to reload config");
            }
        },
        notify::Config::default(),
    )
    .unwrap();

    watcher
        .watch(&config_path.parent().unwrap(), RecursiveMode::NonRecursive)
        .unwrap();

    run(config_rw_lock);
}

fn run(config_rw_lock: Arc<RwLock<cfg::Config>>) {
    let mut id = None;
    loop {
        let config = config_rw_lock.read().unwrap();
        let duration = parse_duration::parse(&config.poll_period).expect("Wrong duration");
        let percent = battery::percentage().unwrap();
        for message in filter_messages(&config.messages, percent, battery::status().unwrap()) {
            let mut notification = build_notification(message, percent);
            if let Some(id) = id {
                notification.id(id);
            }
            let handle = notification.show().expect("Problem showing notification");
            id = Some(handle.id());
        }
        drop(config);
        thread::sleep(duration)
    }
}

#[derive(Parser, Debug)]
#[command[version, about, long_about = None]]
struct Args {
    #[arg(short, long, default_value = cfg::default_config_path().expect("Failed to assign default config path").into_os_string())]
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
