#[macro_use]
extern crate rocket;

use std::{fmt, io};

use rocket::http::Status as HttpStatus;
use rocket::response::{content, Flash, Redirect, Responder};
use rocket::{tokio, Request};

pub mod config;
pub mod db;

pub mod form;
pub mod models;
pub mod string_ext;
mod templates;
pub mod web;

#[derive(Debug)]
pub enum FediurlError {
    Database(sqlx::Error),
    Task(tokio::task::JoinError),
    /// HTTP client error
    Http(reqwest::Error),
    LimitReached,
    Io(io::Error),
    Url(url::ParseError),
    /// Path is invalid or not found
    InvalidPath,
}

#[derive(Responder)]
pub enum RespondOrRedirect {
    Html(content::RawHtml<String>),
    Redirect(Redirect),
    FlashRedirect(Flash<Redirect>),
}

impl From<sqlx::Error> for FediurlError {
    fn from(err: sqlx::Error) -> Self {
        FediurlError::Database(err)
    }
}

impl From<tokio::task::JoinError> for FediurlError {
    fn from(err: tokio::task::JoinError) -> Self {
        FediurlError::Task(err)
    }
}

impl From<reqwest::Error> for FediurlError {
    fn from(err: reqwest::Error) -> Self {
        FediurlError::Http(err)
    }
}

impl From<url::ParseError> for FediurlError {
    fn from(err: url::ParseError) -> Self {
        FediurlError::Url(err)
    }
}

impl From<io::Error> for FediurlError {
    fn from(err: io::Error) -> Self {
        FediurlError::Io(err)
    }
}

impl fmt::Display for FediurlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FediurlError::Database(err) => err.fmt(f),
            FediurlError::Task(err) => err.fmt(f),
            FediurlError::LimitReached => f.write_str("a resource limit was reached"),
            FediurlError::Io(err) => err.fmt(f),
            FediurlError::InvalidPath => f.write_str("invalid path"),
            FediurlError::Http(err) => err.fmt(f),
            FediurlError::Url(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for FediurlError {}

/// Render a template as HTML
pub fn html<T: markup::Render + fmt::Display>(template: T) -> content::RawHtml<String> {
    content::RawHtml(template.to_string())
}

impl<'r> Responder<'r, 'static> for FediurlError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        // TODO: Produce a better error than 500 for ImageError
        match self {
            FediurlError::Database(sqlx::Error::RowNotFound) | FediurlError::InvalidPath => {
                Err(HttpStatus::NotFound)
            }
            _ => {
                error!("{}: {}", req.uri(), self);
                sentry::capture_error(&self);
                Err(HttpStatus::InternalServerError)
            }
        }
    }
}

use serde::{Deserialize, Serialize};

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

// pub async fn rewrite_url(remote_url: &Url) -> eyre::Result<Option<Url>> {
//     // Do a search for the url and pick one as the result
//     let config = Config::read(None)?;
//     let client = Client::builder()
//         .user_agent(format!("Fediurl {}", env!("CARGO_PKG_VERSION")))
//         .build()?;
//
//     let instance = config.instance_url()?;
//     let mut url = instance.join("/api/v2/search")?;
//     let bearer_token = format!("Bearer {}", config.access_token);
//     url.query_pairs_mut()
//         .append_pair("q", remote_url.as_str())
//         .append_pair("resolve", "true");
//
//     // Fetch search results
//     info!("Searching...");
//     let resp = client
//         .get(url.clone())
//         .header(AUTHORIZATION, &bearer_token)
//         .send()
//         .await?;
//     let results: Search = json_or_error(resp).await?;
//
//     // Pick a result, favouring statuses first
//     // TODO: Perhaps there needs to be a hint as whether we're expecting an account or status
//     results
//         .statuses
//         .first()
//         .map(|status| {
//             let acct = format!("@{}", status.account.acct);
//             let mut url = instance.clone();
//             // NOTE(unwrap): won't panic as instance URL is known to be valid as a base URL
//             url.path_segments_mut()
//                 .unwrap()
//                 .extend(&[&acct, &status.id]);
//             Ok(url)
//         })
//         .or_else(|| {
//             // Try accounts
//             results
//                 .accounts
//                 .first()
//                 .map(|account| instance.join(&format!("@{}", account.acct)))
//         })
//         .transpose()
//         .map_err(Report::from)
// }
