mod battery;
mod cfg;

use clap::Parser;
use log::{error, info};
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
    env_logger::init();

    let args = Args::parse();
    if !args.config.exists() {
        error!(
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
        move |res: Result<Event, Error>| match res {
            Ok(event) => {
                if event.kind.is_modify() {
                    match try_to_reload_config_n_times(
                        &cloned_config_path,
                        &cloned_config_rw_lock,
                        10,
                    ) {
                        Ok(_) => info!("Config reloaded successfully"),
                        Err(e) => error!("{:#?}", e),
                    }
                }
            }
            Err(e) => error!("{:#?}", e),
        },
        notify::Config::default(),
    )
    .expect("Unable to start config hot reload");

    watcher
        .watch(
            &config_path
                .parent()
                .expect("How config file cannot have a parent?"),
            RecursiveMode::NonRecursive,
        )
        .expect("Unable to start config hot reload");

    run(config_rw_lock);
}

fn try_to_reload_config_n_times(
    config_path: &PathBuf,
    config_rw_lock: &Arc<RwLock<cfg::Config>>,
    n: usize,
) -> Result<(), String> {
    let mut err = String::new();
    for _ in 0..n {
        match cfg::reload(config_path, config_rw_lock) {
            Ok(_) => return Ok(()),
            Err(cfg::ReloadError::Load(cfg::LoadError::Read(_))) => {
                thread::sleep(Duration::from_millis(10))
            }
            Err(e) => err.push_str(&format!("{:#?}", e)),
        }
    }
    err.push_str("Failed to reload config");
    Err(err)
}

fn run(config_rw_lock: Arc<RwLock<cfg::Config>>) {
    let mut id = None;
    loop {
        let config = config_rw_lock.read().expect("Unable to read config");
        let duration = match parse_duration::parse(&config.poll) {
            Ok(d) => d,
            Err(e) => {
                error!(
                    "Unable to parse poll duration, fallback to default 2 minutes: {:#?}",
                    e
                );
                Duration::from_secs(2 * 60)
            }
        };
        match (battery::percentage(), battery::status()) {
            (Ok(percent), Ok(status)) => {
                for message in filter_messages(&config.messages, percent, status) {
                    let mut notification = match build_notification(message, percent) {
                        Ok(n) => n,
                        Err(e) => {
                            error!("Unable to build a notification: {:#?}", e);
                            continue;
                        }
                    };
                    if let Some(id) = id {
                        notification.id(id);
                    }
                    let handle = match notification.show() {
                        Ok(h) => h,
                        Err(e) => {
                            error!("Unable to show a notification: {:#?}", e);
                            continue;
                        }
                    };
                    id = Some(handle.id());
                }
            }
            (Err(ep), Err(es)) => {
                error!(
                    "Unable to get battery percentage and battery status: {:#?} {:#?}",
                    ep, es
                )
            }
            (Err(ep), _) => {
                error!("Unable to get battery percentage: {:#?}", ep)
            }
            (_, Err(es)) => error!("Unable to get battery status: {:#?}", es),
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

#[derive(Debug)]
enum BuildNotificationError {
    ParseUrgency(ParseUrgencyError),
}

fn build_notification(
    message: &cfg::Message,
    percent: i64,
) -> Result<Notification, BuildNotificationError> {
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
        let urgency =
            parse_urgency(urgency).map_err(|e| BuildNotificationError::ParseUrgency(e))?;
        notification.urgency(urgency);
    }
    Ok(notification)
}

fn format(string: &str, percent: i64) -> String {
    string.replace("{percent}", &percent.to_string())
}

#[derive(Debug)]
struct ParseUrgencyError {}

fn parse_urgency(urgency: &str) -> Result<Urgency, ParseUrgencyError> {
    match urgency {
        "low" | "Low" => Ok(Urgency::Low),
        "normal" | "Normal" => Ok(Urgency::Normal),
        "critical" | "Critical" => Ok(Urgency::Critical),
        _ => Err(ParseUrgencyError {}),
    }
}

fn filter_messages(
    messages: &HashMap<String, cfg::Message>,
    battery_percent: i64,
    status: battery::Status,
) -> Vec<&cfg::Message> {
    messages
        .iter()
        .map(|(_, m)| m)
        .filter(|m| match m.status.parse::<battery::Status>() {
            Ok(s) => s == status,
            Err(e) => {
                error!("Wrong status in message {:#?}, {:#?}", m, e);
                false
            }
        })
        .filter(|m| {
            if m.from > m.to {
                error!("`from` cannot be greater than `to` in message {:#?}", m);
                false
            } else {
                m.from <= battery_percent && battery_percent <= m.to
            }
        })
        .collect()
}
