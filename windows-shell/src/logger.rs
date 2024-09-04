use std::env;

use anyhow::Context;
use log::LevelFilter;
use log4rs::{config::Root, Config};
use windows::Win32::System::Console::AllocConsole;

use crate::util::get_dll_path;

unsafe fn create_console() -> anyhow::Result<()> {
    AllocConsole()?;
    Ok(())
}

pub fn init() -> anyhow::Result<()> {
    if env::var("PINGVIN_SHARE_SHELL_CONSOLE")
        .ok()
        .as_ref()
        .map(String::as_str)
        == Some("1")
    {
        unsafe { create_console()? };
    }

    let config_path = get_dll_path()
        .parent()
        .context("missing exe parent")?
        .to_owned()
        .join("log4rs-shell.yml");

    println!(
        "log4rs config path: {}.\nLogging enabled: {}",
        config_path.display(),
        if config_path.exists() { "yes" } else { "no" }
    );
    if config_path.exists() {
        log4rs::init_file(config_path, Default::default())?;
    } else {
        let root = Root::builder().build(LevelFilter::Off);
        log4rs::init_config(Config::builder().build(root)?)?;
    }

    Ok(())
}
