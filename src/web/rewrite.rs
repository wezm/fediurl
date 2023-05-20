use reqwest::header::AUTHORIZATION;
use rocket::http::uri::Origin;
use rocket::response::{Flash, Redirect};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{http, Route};
use rocket_db_pools::Connection;
use url::Url;

use crate::db::Db;
use crate::models::instance::Instance;
use crate::web::session::AuthenticatedUser;
use crate::{http_client, web};
use crate::{json_or_error, ErrorResponse, FediurlError, RespondOrRedirect};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Search {
    accounts: Vec<Account>,
    statuses: Vec<Status>,
    // hashtags is also present but we're not interested in those
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Account {
    id: String,
    username: String,
    acct: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct Status {
    id: String,
    account: Account,
}

pub fn routes() -> Vec<Route> {
    routes![rewrite, rewrite_json]
}

/// URL rewrite endpoint, will redirect to equivalent URL on user's instance
#[get("/https:/<_..>", rank = 2)]
async fn rewrite(
    mut db: Connection<Db>,
    // config: &State<AppConfig>,
    user: AuthenticatedUser,
    origin: &Origin<'_>,
) -> Result<RespondOrRedirect, FediurlError> {
    match lookup(&mut db, &user, origin).await {
        Ok(Some(url)) => Ok(RespondOrRedirect::Redirect(Redirect::to(url.to_string()))),
        // not found
        Ok(None) => Err(FediurlError::InvalidPath), // TODO: Show no match page
        Err(err) => Ok(RespondOrRedirect::FlashRedirect(Flash::error(
            Redirect::to(uri!(web::home)),
            format!("Error: {}", err),
        ))),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Rewrite {
    destination: String,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde", tag = "type")]
enum RewriteResponse {
    Redirect(Rewrite),
    Error(ErrorResponse),
}

// URL rewrite endpoint, JSON version
#[get("/https:/<_..>", format = "json")]
async fn rewrite_json(
    mut db: Connection<Db>,
    // config: &State<AppConfig>,
    user: AuthenticatedUser,
    origin: &Origin<'_>,
) -> Json<RewriteResponse> {
    match lookup(&mut db, &user, origin).await {
        Ok(Some(url)) => Json(RewriteResponse::Redirect(Rewrite {
            destination: url.to_string(),
        })),
        // TODO: Set status to something other than 200 for these?
        Ok(None) => Json(RewriteResponse::Error(ErrorResponse {
            status: http::Status::NotFound.code,
            error: "no_match".to_string(),
            error_description: "No matching URL found".to_string(),
        })),
        Err(err) => {
            let resp = match err {
                FediurlError::Database(err) => ErrorResponse {
                    status: http::Status::InternalServerError.code,
                    error: "database".to_string(),
                    error_description: err.to_string(),
                },
                FediurlError::Http(err) => ErrorResponse {
                    status: http::Status::InternalServerError.code,
                    error: "http_client".to_string(),
                    error_description: err.to_string(),
                },
                FediurlError::Io(err) => ErrorResponse {
                    status: http::Status::InternalServerError.code,
                    error: "io".to_string(),
                    error_description: err.to_string(),
                },
                FediurlError::Url(err) => ErrorResponse {
                    status: http::Status::BadRequest.code,
                    error: "invalid_url".to_string(),
                    error_description: err.to_string(),
                },
                FediurlError::InvalidPath => ErrorResponse {
                    status: http::Status::NotFound.code,
                    error: "invalid_path".to_string(),
                    error_description: "path or URL was invalid or not found".to_string(),
                },
                FediurlError::ErrorResponse(err) => err,
            };
            Json(RewriteResponse::Error(resp))
        }
    }
}

async fn lookup(
    db: &mut Connection<Db>,
    user: &AuthenticatedUser,
    origin: &Origin<'_>,
) -> Result<Option<Url>, FediurlError> {
    let instance = Instance::from_id(&mut *db, user.instance_id).await?;
    let client = http_client()?;

    // Build the remote_url
    let remote_url = &origin.to_string()[1..]; // skip leading slash

    // Perform search to try to find URL on user's instance
    let mut url = instance.url().join("/api/v2/search")?;
    let bearer_token = format!("Bearer {}", user.access_token);
    url.query_pairs_mut()
        .append_pair("q", &remote_url)
        .append_pair("resolve", "true");

    // Fetch search results
    let resp = client
        .get(url.clone())
        .header(AUTHORIZATION, &bearer_token)
        .send()
        .await?;
    let results = json_or_error::<Search>(resp).await?;

    // Pick a result, favouring statuses first
    // TODO: Perhaps there needs to be a hint as whether we're expecting an account or status
    results
        .statuses
        .first()
        .map(|status| {
            let acct = format!("@{}", status.account.acct);
            let mut url = instance.url();
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
                .map(|account| instance.url().join(&format!("@{}", account.acct)))
        })
        .transpose()
        .map_err(FediurlError::from)
}
