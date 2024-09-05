use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{RwLock, RwLockReadGuard},
    thread::{self},
    time::Duration,
};

use anyhow::Context;
use debounce::EventDebouncer;
use notify::{RecursiveMode, Watcher};
use serde::Deserialize;

use crate::util;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ConfigRoot {
    default: ShellConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShellConfig {
    /// Path to the pingvin cli tool
    pub pingvin_exe: Option<PathBuf>,

    /// Additional args for the pingvin executable.
    /// Arguments must be split by ","
    pub pingvin_args: Option<String>,

    /// Menu title to display for context menus
    pub menu_title: Option<String>,

    /// Menu title to display for context menus
    pub menu_icon: Option<String>,
}

static CONFIG_INSTANCE: RwLock<ShellConfig> = RwLock::new(ShellConfig {
    pingvin_exe: None,
    pingvin_args: None,

    menu_title: None,
    menu_icon: None,
});

fn config_watcher_worker(config_path: PathBuf) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(
        &config_path
            .parent()
            .context("missing config path directory")?,
        RecursiveMode::NonRecursive,
    )?;

    let reloader = EventDebouncer::new(Duration::from_secs(5), {
        let config_path = config_path.clone();
        move |_| {
            log::debug!("Config changed. Reloading config.");
            if let Err(err) = load_config_internal(&config_path) {
                log::warn!("Failed to load new config: {}", err);
            }
        }
    });

    for event in rx {
        match event {
            Ok(event) => {
                if event.paths.contains(&config_path) {
                    reloader.put(());
                }
            }
            Err(e) => log::warn!("Config watch error: {:?}", e),
        }
    }
    Ok(())
}

pub fn load_config_internal(path: &Path) -> anyhow::Result<()> {
    let reader = File::open(path)?;
    let config: ConfigRoot = serde_ini::from_read(reader).context("config parse")?;
    *CONFIG_INSTANCE.write().unwrap() = config.default;
    Ok(())
}

pub fn init_config() -> anyhow::Result<()> {
    let config_path = util::get_dll_path()
        .parent()
        .context("missing dll folder")?
        .join("config_shell.ini");

    thread::spawn({
        let config_path = config_path.clone();
        move || {
            if let Err(err) = config_watcher_worker(config_path) {
                log::error!("Config watch worker exited: {}", err);
            }
        }
    });

    if !config_path.exists() {
        log::info!(
            "Using default config. {} does not exists",
            config_path.display()
        );
        return Ok(());
    }

    log::info!("Loading config from {}", config_path.display());
    load_config_internal(&config_path)?;
    Ok(())
}

pub fn current_config() -> RwLockReadGuard<'static, ShellConfig> {
    CONFIG_INSTANCE.read().unwrap()
}
