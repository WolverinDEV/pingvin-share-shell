use std::thread;

use anyhow::Context;
use nsis_plugin_api::*;
use serde::Deserialize;
use url::Url;

/// Build and check the syntax of the server url given from the users input.
/// Args: <server url> <username> <password>
///
/// On success returns 1 and the server URI on failure 0 and an error message
#[nsis_fn]
fn BuildServerUrl() -> Result<(), Error> {
    let server_url = popstr()?;
    let username = popstr()?;
    let password = popstr()?;

    if server_url.is_empty() {
        pushstr("URL empty")?;
        pushint(0)?;
        return Ok(());
    }

    match build_server_url(&server_url, &username, &password) {
        Ok(url) => {
            pushstr(&url)?;
            pushint(1)?;
        }
        Err(err) => {
            pushstr(&format!("{}", err))?;
            pushint(0)?;
        }
    };
    return Ok(());
}

fn build_server_url(server_url: &str, username: &str, password: &str) -> anyhow::Result<String> {
    if server_url.is_empty() {
        anyhow::bail!("URL empty");
    }

    let mut url = Url::parse(&server_url)?;
    if url.username().is_empty() {
        if !username.is_empty() {
            let _ = url.set_username(&username);
        }
    } else if !username.is_empty() {
        anyhow::bail!("multiple username definitions");
    }

    if url.password().is_none() {
        if !password.is_empty() {
            let _ = url.set_password(Some(&password));
        }
    } else if !password.is_empty() {
        anyhow::bail!("multiple password definitions");
    }

    Ok(format!("{}", url))
}

#[nsis_fn]
fn ValidateServerUrl() -> Result<(), Error> {
    let server_url = popstr()?;

    let result = thread::spawn(move || validate_server_url(&server_url))
        .join()
        .unwrap();

    match result {
        Ok((app_name, app_url)) => {
            pushstr(&app_url)?;
            pushstr(&app_name)?;
            pushint(1)?;
        }
        Err(err) => {
            pushstr(&format!("{}", err))?;
            pushint(0)?;
        }
    }

    Ok(())
}

fn validate_server_url(server_url: &str) -> anyhow::Result<(String, String)> {
    #[derive(Deserialize)]
    struct ConfigEntry {
        key: String,
        value: String,
    }

    let response = ureq::get(&format!("{}configs", server_url)).call()?;
    if response.status() != 200 {
        anyhow::bail!("HTTP request failed with status {}", response.status());
    }
    let response = response.into_json::<Vec<ConfigEntry>>()?;

    let app_name = response
        .iter()
        .find(|entry| entry.key == "general.appName")
        .context("missing app name config entry")?;

    let app_url = response
        .iter()
        .find(|entry| entry.key == "general.appUrl")
        .context("missing app url config entry")?;

    Ok((app_name.value.to_string(), app_url.value.to_string()))
}

#[cfg(test)]
mod test {
    use crate::validate_server_url;

    #[test]
    fn test_validate() {
        let result =
            validate_server_url("https://WolverinDEV:thisisnoitmypassword@sendy.did.science/api/");
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}
