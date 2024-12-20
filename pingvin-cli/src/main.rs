#![cfg_attr(target_family = "windows", windows_subsystem = "windows")]

use anyhow::Context;
use api::{ExpireDuration, PingvinApi};
use clap::Parser;
use output::{AppOutput, OutputType};
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

    /// Change the output type on how process indication will be done
    #[arg(short, long, value_enum, default_value_t = OutputType::Console)]
    pub output: OutputType,
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

    let output = output::create(args.output)?;
    let result = execute_upload(&args, &*output).await;
    if let Err(err) = result {
        output.show_upload_error(&err);

        return match args.output {
            OutputType::Console => Ok(ExitCode::FAILURE),

            /* Return success, so the context menu handler does not show an additional popup */
            OutputType::WindowsNotification => Ok(ExitCode::SUCCESS),
        };
    }

    Ok(ExitCode::SUCCESS)
}

async fn execute_upload(args: &Args, output: &dyn AppOutput) -> anyhow::Result<()> {
    let mut server_api = PingvinApi::new(args.server_url.clone())?;

    log::info!("Fetching server config");
    let server_config = server_api.public_config().await.context("server config")?;

    let allow_unauthenticated_shares = server_config
        .get_bool("share.allowUnauthenticatedShares")
        .unwrap_or(false);

    if !allow_unauthenticated_shares || !args.server_url.username().is_empty() {
        let username = args.server_url.username();
        let Some(password) = args.server_url.password() else {
            anyhow::bail!("Unauthenticated shares are not allowed.\nPlease provide a user and a password within the server URL");
        };

        if username.is_empty() {
            anyhow::bail!("Unauthenticated shares are not allowed.\nPlease provide a user and a password within the server URL");
        }

        log::info!("Try to login with given credentials.");
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

    share_builder.with_callback(output.create_upload_handler(&server_config)?);

    let _ = share_builder.upload().await?;
    Ok(())
}
