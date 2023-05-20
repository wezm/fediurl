use std::env;

use rocket::http::uri::Host;
use rocket::serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AppConfig {
    pub hosts: Vec<Host<'static>>,
    pub sentry_dsn: Option<String>,
}

impl Default for AppConfig {
    fn default() -> AppConfig {
        AppConfig {
            hosts: Vec::new(),
            sentry_dsn: None,
        }
    }
}

pub fn git_revision() -> String {
    env::var("FEDIURL_REVISION").unwrap_or_else(|_| String::from("dev"))
}
