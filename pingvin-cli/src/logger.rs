use std::env;

use anyhow::Context;
use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

pub fn init() -> anyhow::Result<()> {
    #[cfg(target_family = "windows")]
    unsafe {
        use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    };

    let config_path = env::current_exe()?
        .parent()
        .context("missing exe parent")?
        .to_owned()
        .join("log4rs.yml");

    println!(
        "> log4rs config path: {}.\n> Logging enabled: {}",
        config_path.display(),
        if config_path.exists() { "yes" } else { "no" }
    );
    if config_path.exists() {
        log4rs::init_file(config_path, Default::default())?;
    } else {
        let config = Config::builder()
            .appender(
                Appender::builder().build(
                    "console",
                    Box::new(
                        ConsoleAppender::builder()
                            .encoder(Box::new(PatternEncoder::new("{h({l}): <5.5} - {m}{n}")))
                            .build(),
                    ),
                ),
            )
            .build(Root::builder().appender("console").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    }

    Ok(())
}
