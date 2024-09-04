use api::{ExpireDuration, PingvinApi};
use clap::Parser;
use output::{create_log_upload_event_handler, create_win_upload_event_handler};
use reqwest::Url;
use std::{path::PathBuf, process::ExitCode};

mod api;
mod logger;
mod output;

/// CLI tool to upload files to a pinving share instance
#[derive(Debug, Parser)]
pub struct Args {
    /// The server URL of the pingvin share to upload the files to.
    #[arg(short, long, value_parser = Url::parse)]
    pub server_url: Url,

    /// A list of files which should be uploaded.
    #[arg(short, long, required = true)]
    pub files: Vec<PathBuf>,

    /// Specify the id the share should create. If a share if that id already exists the file upload will fail.
    #[arg(long)]
    pub id: Option<String>,

    /// Specify the display name for the share
    #[arg(short, long)]
    pub name: Option<String>,

    /// Specify the description for the share
    #[arg(short, long)]
    pub description: Option<String>,

    /// Set the expiration date of the share.  
    /// Format: <amount>-<unit>  
    /// Default: 'never'  
    /// Available units: minutes, hours, days, months, years
    #[arg(short, long, verbatim_doc_comment)]
    pub expire_duration: Option<ExpireDuration>,

    #[arg(short, long)]
    pub console: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    logger::init()?;
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{}", err);
            return Ok(ExitCode::FAILURE);
        }
    };

    let mut server_api = PingvinApi::new(args.server_url.clone())?;
    let server_config = server_api.public_config().await?;

    let allow_unauthenticated_shares = server_config
        .get_bool("share.allowUnauthenticatedShares")
        .unwrap_or(false);

    if !allow_unauthenticated_shares || !args.server_url.username().is_empty() {
        let username = args.server_url.username();
        let Some(password) = args.server_url.password() else {
            anyhow::bail!("Unauthenticated shares are not allowed. Please provide a user and a password within the server URL");
        };

        if username.is_empty() {
            anyhow::bail!("Unauthenticated shares are not allowed. Please provide a user and a password within the server URL");
        }

        log::info!("Logging into share");
        if !server_api.login(username, password).await? {
            anyhow::bail!("Failed to login with the given credentials.");
        }
    }

    let mut share_builder = server_api.create_share();
    if let Some(value) = &args.id {
        share_builder.set_id(value.to_string());
    }
    if let Some(value) = &args.name {
        share_builder.set_name(value.to_string());
    }
    if let Some(value) = &args.description {
        share_builder.set_description(value.to_string());
    }
    for file in &args.files {
        share_builder.add_file(file.to_owned());
    }

    if args.console {
        share_builder.with_callback(create_log_upload_event_handler(&server_config));
    } else {
        share_builder.with_callback(create_win_upload_event_handler(&server_config)?);
    }

    let _ = share_builder.upload().await?;
    Ok(ExitCode::SUCCESS)
}
