mod battery;
mod cfg;

use clap::Parser;
use env_logger::Env;
use log::{error, info};
use notify::{
    event::ModifyKind, Error, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use notify_rust::{Notification, Urgency};
use std::{
    collections::HashMap,
    fmt::Display,
    path::PathBuf,
    process,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};

#[derive(Parser, Debug)]
#[command[version, about, long_about = None]]
struct Args {
    #[arg(
        short,
        long,
        default_value = cfg::default_config_path().unwrap().into_os_string(),
        help = "Config path"
    )]
    config: PathBuf,
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
    #[arg(long, help = "Disable config auto reload")]
    disable_autoreload: bool,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    } else {
        env_logger::init();
    }

    let config_path = args.config;
    let config_rw_lock = Arc::new(RwLock::new(cfg::load(&config_path).unwrap_or_else(|e| {
        error!("{}", e);
        process::exit(1);
    })));

    if !args.disable_autoreload {
        run_with_config_autoreload(&config_path, config_rw_lock)
    } else {
        run(config_rw_lock);
    }
}

fn run_with_config_autoreload(config_path: &PathBuf, config_rw_lock: Arc<RwLock<cfg::Config>>) {
    let runner = {
        let config_rw_lock = config_rw_lock.clone();
        thread::spawn(|| run(config_rw_lock))
    };
    let runner_thread = runner.thread();
    let mut watcher = {
        let config_path = config_path.clone();
        let config_rw_lock = config_rw_lock.clone();
        let runner_thread = runner_thread.clone();
        RecommendedWatcher::new(
            move |res: Result<Event, Error>| match res {
                Ok(event) => {
                    if let EventKind::Modify(ModifyKind::Data(_)) = event.kind {
                        match try_to_reload_config_n_times(&config_path, &config_rw_lock, 10) {
                            Ok(_) => {
                                runner_thread.unpark();
                                info!("Config reloaded")
                            }
                            Err(e) => error!("{}", e),
                        }
                    }
                }
                Err(e) => error!("{}", e),
            },
            notify::Config::default(),
        )
        .unwrap_or_else(|e| {
            error!("Unable to start config autoreload: {}", e);
            process::exit(1)
        })
    };

    let config_dir = config_path.parent().unwrap_or_else(|| {
        error!("Unable to get a directory the config file is in");
        process::exit(1)
    });
    watcher
        .watch(config_dir, RecursiveMode::NonRecursive)
        .unwrap_or_else(|e| {
            error!("Unable to start config autoreload: {}", e);
            process::exit(1)
        });
    info!("Config autoreload started");

    runner.join().expect("Runner panicked");
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
            Err(e) => err.push_str(&format!("{}", e)),
        }
    }
    err.push_str("Failed to reload config");
    Err(err)
}

fn run(config_rw_lock: Arc<RwLock<cfg::Config>>) {
    info!("Battery-friend started");
    let mut ids = HashMap::new();
    loop {
        let config = config_rw_lock.read().expect("Unable to read config");
        let duration = parse_duration::parse(&config.poll).unwrap_or_else(|e| {
            error!(
                "Unable to parse poll duration, fallback to default 2 minutes: {}",
                e
            );
            Duration::from_secs(2 * 60)
        });

        match (battery::percentage(), battery::status()) {
            (Ok(percent), Ok(status)) => {
                for (name, message) in filter_messages(&config.messages, percent, status) {
                    let mut notification = match build_notification(message, percent) {
                        Ok(n) => n,
                        Err(e) => {
                            error!("Unable to build a notification: {}", e);
                            continue;
                        }
                    };
                    if let Some(id) = ids.get(name) {
                        notification.id(*id);
                    }
                    let handle = match notification.show() {
                        Ok(h) => h,
                        Err(e) => {
                            error!("Unable to show a notification: {}", e);
                            continue;
                        }
                    };

                    if !ids.contains_key(name) {
                        ids.insert(name.clone(), handle.id());
                    }
                }
            }
            (Err(ep), Err(es)) => {
                error!(
                    "Unable to get battery percentage and battery status: {} {}",
                    ep, es
                )
            }
            (Err(ep), _) => {
                error!("Unable to get battery percentage: {}", ep)
            }
            (_, Err(es)) => error!("Unable to get battery status: {}", es),
        }

        ids.retain(|name, _| config.messages.contains_key(name));
        drop(config);
        thread::park_timeout(duration)
    }
}

#[derive(Debug)]
enum BuildNotificationError {
    ParseUrgency(ParseUrgencyError),
}

impl Display for BuildNotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let m = "Unable to build notification";
        match self {
            BuildNotificationError::ParseUrgency(e) => write!(f, "{}: {}", m, e),
        }
    }
}

impl std::error::Error for BuildNotificationError {}

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
struct ParseUrgencyError {
    s: String,
}

impl Display for ParseUrgencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error parsing notification urgency. Unknown urgency `{}`",
            self.s
        )
    }
}

fn parse_urgency(urgency: &str) -> Result<Urgency, ParseUrgencyError> {
    match urgency {
        "low" | "Low" => Ok(Urgency::Low),
        "normal" | "Normal" => Ok(Urgency::Normal),
        "critical" | "Critical" => Ok(Urgency::Critical),
        _ => Err(ParseUrgencyError {
            s: urgency.to_owned(),
        }),
    }
}

fn filter_messages(
    messages: &HashMap<String, cfg::Message>,
    battery_percent: i64,
    status: battery::Status,
) -> Vec<(&String, &cfg::Message)> {
    messages
        .iter()
        .filter(|(_, m)| match m.status.parse::<battery::Status>() {
            Ok(s) => s == status,
            Err(e) => {
                error!("Wrong status in message {:#?}, {}", m, e);
                false
            }
        })
        .filter(|(_, m)| {
            if m.from > m.to {
                error!("`from` cannot be greater than `to` in message {:#?}", m);
                false
            } else {
                m.from <= battery_percent && battery_percent <= m.to
            }
        })
        .collect()
}
