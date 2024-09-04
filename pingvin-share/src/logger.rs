use std::env;

use anyhow::Context;
use log::LevelFilter;
use log4rs::{config::Root, Config};

pub fn init() -> anyhow::Result<()> {
    let config_path = env::current_exe()?
        .parent()
        .context("missing exe parent")?
        .to_owned()
        .join("log4rs.yml");

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
