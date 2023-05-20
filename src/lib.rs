#[macro_use]
extern crate rocket;

use std::{fmt, io};

use reqwest::{Client, StatusCode};
use rocket::http::Status as HttpStatus;
use rocket::response::{content, Flash, Redirect, Responder};
use rocket::serde::{Deserialize, DeserializeOwned, Serialize};
use rocket::Request;

pub mod config;
pub mod db;

pub mod form;
pub mod models;
pub mod string_ext;
mod templates;
pub mod web;

pub const NAME: &str = "Fediurl";

#[derive(Debug)]
pub enum FediurlError {
    Database(sqlx::Error),
    /// HTTP client error
    Http(reqwest::Error),
    Io(io::Error),
    Url(url::ParseError),
    /// Path is invalid or not found
    InvalidPath,
    /// An error response from a Mastodon instance
    ErrorResponse(ErrorResponse),
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
            FediurlError::Io(err) => err.fmt(f),
            FediurlError::InvalidPath => f.write_str("invalid path"),
            FediurlError::Http(err) => err.fmt(f),
            FediurlError::Url(err) => err.fmt(f),
            FediurlError::ErrorResponse(err) => f.write_str(&err.error_description),
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

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct MastodonErrorResponse {
    pub error: String,
    pub error_description: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ErrorResponse {
    #[serde(skip_deserializing)]
    pub status: u16,
    pub error: String,
    pub error_description: String,
}

impl ErrorResponse {
    fn new(status: StatusCode, err: MastodonErrorResponse) -> Self {
        Self {
            status: status.as_u16(),
            error: err.error,
            error_description: err.error_description,
        }
    }
}

pub(crate) async fn json_or_error<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, FediurlError> {
    if response.status().is_success() {
        let app = response.json().await?;
        Ok(app)
    } else {
        let status = response.status();
        // TODO: Distinguish 4xx and 5xx responses
        let err = response
            .json::<MastodonErrorResponse>()
            .await
            .map(|err| ErrorResponse::new(status, err))
            .unwrap_or_else(|_| ErrorResponse {
                status: status.as_u16(),
                error: "http_client".to_string(),
                error_description: "Request to instance was unsuccessful.".to_string(),
            });
        Err(FediurlError::ErrorResponse(err))
    }
}

pub(crate) fn http_client() -> reqwest::Result<Client> {
    Client::builder()
        .user_agent(format!("{} {}", NAME, env!("CARGO_PKG_VERSION")))
        .build()
}
