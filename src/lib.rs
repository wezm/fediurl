mod config;
#[cfg(windows)]
mod dirs;
#[cfg(not(windows))]
mod xdg;

#[cfg(not(windows))]
use crate::xdg as dirs;

use std::io::Write;

use std::{env, io};

use eyre::eyre;
use log::{debug, error, info};
use reqwest::header::AUTHORIZATION;
use reqwest::{Client, Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use simple_eyre::{eyre, Report};

use crate::config::Config;

const SCOPES: &str = "read:search";

#[derive(Deserialize)]
struct ErrorResponse {
    // error: String,
    error_description: String,
}

#[derive(Deserialize)]
struct Application {
    name: String,
    // website: Option<String>,
    // vapid_key: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize, Serialize)]
struct Search {
    accounts: Vec<Account>,
    statuses: Vec<Status>,
    // hashtags is also present but we're not interested in that
}

#[derive(Deserialize, Serialize)]
struct Account {
    id: String,
    username: String,
    acct: String,
}

#[derive(Deserialize, Serialize)]
struct Status {
    id: String,
    account: Account,
}

/// Perform the OAuth flow to obtain credentials
pub async fn auth(instance: Url) -> eyre::Result<()> {
    let client = Client::builder()
        .user_agent(format!("Fediurl {}", env!("CARGO_PKG_VERSION")))
        .build()?;

    // Register application to obtain client id and secret
    let url = instance.join("/api/v1/apps")?;
    let resp = client
        .post(url)
        .form(&[
            ("client_name", "Fediurl"),
            ("redirect_uris", "urn:ietf:wg:oauth:2.0:oob"),
            ("scopes", SCOPES),
            // ("website", instance.as_str()) // There is no website for this application
        ])
        .send()
        .await?; // TODO: Add context info to error
    let app: Application = json_or_error(resp).await?;

    let client_id = app
        .client_id
        .ok_or_else(|| eyre!("app response is missing client id"))?;
    let client_secret = app
        .client_secret
        .ok_or_else(|| eyre!("app response is missing client secret"))?;
    debug!("Got application: {}, ID: {}", app.name, client_id);

    // Show the approval page
    let mut url = instance.join("/oauth/authorize")?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", "urn:ietf:wg:oauth:2.0:oob")
        .append_pair("scope", SCOPES);
    // TODO: Use browser opener to open this URL
    println!(
        "\nOpen this page in your browser and paste the code:\n{}",
        url
    );
    print!("\nCode: ");
    io::stdout().flush()?;
    let mut code = String::new();
    io::stdin().read_line(&mut code)?;

    let code = code.trim();
    if code.is_empty() {
        return Err(eyre!("code is required"));
    }

    // Use client id, secret, and code to get a token
    let url = instance.join("/oauth/token")?;
    let resp = client
        .post(url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id.as_str()),
            ("client_secret", &client_secret),
            ("redirect_uri", "urn:ietf:wg:oauth:2.0:oob"),
            ("scope", SCOPES),
        ])
        .send()
        .await?; // TODO: Add context info to error
    let token_resp: TokenResponse = json_or_error(resp).await?;
    debug!("Got token");

    // Save the token (and client credentials)
    let config = Config::new(
        client_id,
        client_secret,
        instance.to_string(),
        token_resp.access_token,
    );
    Config::create(None, config)?; // TODO: Support custom config path
    debug!("Saved config"); // FIXME: Output a success message

    Ok(())
}

pub async fn rewrite_url(remote_url: &Url) -> eyre::Result<Option<Url>> {
    // Do a search for the url and pick one as the result
    let config = Config::read(None)?;
    let client = Client::builder()
        .user_agent(format!("Fediurl {}", env!("CARGO_PKG_VERSION")))
        .build()?;

    let instance = config.instance_url()?;
    let mut url = instance.join("/api/v2/search")?;
    let bearer_token = format!("Bearer {}", config.access_token);
    url.query_pairs_mut()
        .append_pair("q", remote_url.as_str())
        .append_pair("resolve", "true");

    // Fetch search results
    info!("Searching...");
    let resp = client
        .get(url.clone())
        .header(AUTHORIZATION, &bearer_token)
        .send()
        .await?;
    let results: Search = json_or_error(resp).await?;

    // Pick a result, favouring statuses first
    // TODO: Perhaps there needs to be a hint as whether we're expecting an account or status
    results
        .statuses
        .first()
        .map(|status| {
            let acct = format!("@{}", status.account.acct);
            let mut url = instance.clone();
            // NOTE(unwrap): won't panic as instance URL is known to be valid as a base URL
            url.path_segments_mut()
                .unwrap()
                .extend(&[&acct, &status.id]);
            Ok(url)
        })
        .or_else(|| {
            // Try accounts
            results
                .accounts
                .first()
                .map(|account| instance.join(&format!("@{}", account.acct)))
        })
        .transpose()
        .map_err(Report::from)
}

async fn json_or_error<T: DeserializeOwned>(response: Response) -> eyre::Result<T> {
    if response.status().is_success() {
        let app = response.json().await?;
        Ok(app)
    } else {
        error!("Request was unsuccessful ({})", response.status().as_u16());
        // TODO: Distinguish 4xx and 5xx responses
        let err: ErrorResponse = response.json().await?;
        Err(eyre!(err.error_description))
    }
}
