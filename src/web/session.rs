//! User authentication/session management.

use reqwest::{Client, Response, Url};
use std::ops::Deref;

// TODO: Refresh session cookie on new requests

use rocket::form::{Context, Contextual, Form};
use rocket::http::uri::{Absolute, Host};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::{try_outcome, IntoOutcome};
use rocket::request::{FlashMessage, FromRequest, Outcome, Request};
use rocket::response::content::RawHtml;
use rocket::response::{Flash, Redirect};
use rocket::serde::Deserialize;
use rocket::{Route, State};
use rocket_db_pools::Connection;
use serde::de::DeserializeOwned;
use time::Duration; // for Cookie

use crate::config::AppConfig;
use crate::db::Db;
use crate::form::{validate, ContextExt, NonEmptyString};
use crate::models::instance::{Instance, NewInstance};
use crate::models::user::{NewUser, User};
use crate::templates::{self, Layout, Title};
use crate::web::XForwardedProto;
use crate::{html, web, FediurlError, RespondOrRedirect};

pub const FEDIURL_SESSION: &str = "FEDIURL_SESSION";
const SCOPES: &str = "read:search";
const FEDIURL_WEBSITE: &str = "https://fediurl.7bit.org/";

pub struct AuthenticatedUser(User);

#[derive(FromForm)]
struct LoginForm<'v> {
    #[field(validate=validate::domain().map(drop))]
    instance: NonEmptyString<'v>,
}

#[derive(Deserialize)]
struct Application {
    name: String,
    // website: Option<String>,
    // vapid_key: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Debug)]
pub enum AuthenticatedUserError {
    Database(sqlx::Error),
    GuardFailure,
}

pub fn routes() -> Vec<Route> {
    routes![new, new_redirect, create, delete, auth]
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = AuthenticatedUserError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // TODO: use the request local state
        // https://api.rocket.rs/v0.5-rc/rocket/request/trait.FromRequest.html#request-local-state
        let mut db = try_outcome!(request
            .guard::<Connection<Db>>()
            .await
            .map_failure(|(status, _)| (status, AuthenticatedUserError::GuardFailure)));

        let user_id = try_outcome!(request
            .cookies()
            .get_private(FEDIURL_SESSION)
            .and_then(|cookie| cookie.value().parse().ok())
            .or_forward(()));

        User::from_id(&mut *db, user_id)
            .await
            .map(AuthenticatedUser)
            .map_err(|err| err.into())
            .or_forward(())
    }
}

impl Deref for AuthenticatedUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[get("/login")]
fn new_redirect(_user: AuthenticatedUser) -> Redirect {
    // Already logged in
    Redirect::to(uri!(web::home))
}

#[get("/login", rank = 2)]
pub async fn new(
    config: &State<AppConfig>,
    flash: Option<FlashMessage<'_>>,
) -> Result<RawHtml<String>, FediurlError> {
    let body = templates::session::New {
        context: &Context::default(),
    };

    let page = Layout {
        config: config,
        title: Title::head_and_body("Log in"),
        flash: flash.as_ref(),
        current_user: None,
        head: templates::Nil {},
        body,
    };
    Ok(html(page))
}

#[post("/login", data = "<form>")]
async fn create(
    host: &Host<'_>,
    proto: Option<XForwardedProto<'_>>,
    mut db: Connection<Db>,
    config: &State<AppConfig>,
    cookies: &CookieJar<'_>,
    form: Form<Contextual<'_, LoginForm<'_>>>,
) -> Result<RespondOrRedirect, FediurlError> {
    match form.value {
        // Form was valid, try logging the user in
        Some(ref submission) => {
            let instance = Instance::from_domain_optional(&mut *db, &submission.instance).await;

            match instance {
                Ok(Some(instance)) => {
                    // Instance already exists so we can redirect to the auth page directly
                    // Determine the host we're running on
                    let prefix = safe_host(host, &proto, &config);
                    let redirect_uri =
                        uri!(prefix, auth(domain = &instance.domain, code = _)).to_string();
                    let mut auth_url = instance.url().join("/oauth/authorize")?;
                    auth_url
                        .query_pairs_mut()
                        .append_pair("response_type", "code")
                        .append_pair("client_id", &instance.client_id)
                        .append_pair("redirect_uri", &redirect_uri)
                        .append_pair("scope", SCOPES);
                    Ok(RespondOrRedirect::Redirect(Redirect::to(
                        auth_url.to_string(),
                    )))
                }
                Ok(None) => {
                    // This is a newly encountered instance
                    // TODO: Extract method for building a client
                    let client = Client::builder()
                        .user_agent(format!("Fediurl {}", env!("CARGO_PKG_VERSION"))) // TODO: Move Fediurl into a constant
                        .build()?;

                    let domain = &*submission.instance;
                    let instance_url = Url::parse(&format!("https://{}/", domain))?;

                    // Register application to obtain client id and secret
                    let prefix = safe_host(host, &proto, &config);
                    let redirect_uri = uri!(prefix, auth(domain = domain, code = _)).to_string();
                    let url = instance_url.join("/api/v1/apps")?;
                    let resp = client
                        .post(url)
                        .form(&[
                            ("client_name", "Fediurl"),       // TODO: Move Fediurl into a constant
                            ("redirect_uris", &redirect_uri), // TODO: This needs to be space separated
                            ("scopes", SCOPES),
                            ("website", FEDIURL_WEBSITE),
                        ])
                        .send()
                        .await?; // TODO: Add context info to error
                    let Ok(app) = json_or_error::<Application>(resp).await else {
                         todo!("render the error if it's an ErrorResponse");
                    };

                    let (Some(client_id), Some(client_secret)) = (app.client_id, app.client_secret) else {
                        todo!("Render form again with flash message")
                    };
                    debug!("Got application: {}, ID: {}", app.name, client_id);

                    let new_instance = NewInstance {
                        domain: submission.instance.to_string(),
                        client_id,
                        client_secret,
                    };

                    let instance_id = Instance::create(&mut *db, new_instance).await?;
                    let instance = Instance::from_id(&mut *db, instance_id).await?;

                    // TODO: Extract method
                    let mut auth_url = instance.url().join("/oauth/authorize")?;
                    auth_url
                        .query_pairs_mut()
                        .append_pair("response_type", "code")
                        .append_pair("client_id", &instance.client_id)
                        .append_pair("redirect_uri", &redirect_uri)
                        .append_pair("scope", SCOPES);
                    Ok(RespondOrRedirect::Redirect(Redirect::to(
                        auth_url.to_string(),
                    )))
                }
                Err(err) => Err(FediurlError::from(err).into()),
            }
        }
        // Form was not valid, re-render the login page (with errors)
        None => {
            let flash = Flash::error(
                cookies,
                format!(
                    "Unable to log in. Check these fields for errors: {}",
                    form.context.fields_with_errors()
                ),
            );
            render_new(config, flash, &form.context)
        }
    }
}

fn safe_host<'r>(
    host: &'r Host<'r>,
    proto: &'r Option<XForwardedProto<'r>>,
    config: &'r AppConfig,
) -> Absolute<'r> {
    let scheme = proto.map(|proto| proto.0).unwrap_or("http");
    host.to_absolute(scheme, &config.hosts)
        .expect("FIXME: flash redirect with error message")
}

fn render_new<'v>(
    config: &AppConfig,
    flash: FlashMessage<'_>,
    context: &Context<'v>,
) -> Result<RespondOrRedirect, FediurlError> {
    let body = templates::session::New { context };

    let page = Layout {
        config: config,
        title: Title::head_and_body("Log in"),
        flash: Some(&flash),
        current_user: None,
        head: templates::Nil {},
        body,
    };
    Ok(RespondOrRedirect::Html(html(page)))
}

#[delete("/logout")]
fn delete(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    cookies.remove_private(Cookie::named(FEDIURL_SESSION));
    Flash::success(Redirect::to(uri!(web::home)), "You have been logged out")
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// OAuth authentication callback endpoint
#[get("/auth/<domain>?<code>")]
async fn auth(
    // This is only optional to allow uri generation without the code query parameter. All actual
    // request require the parameter to be present.
    host: &Host<'_>,
    proto: Option<XForwardedProto<'_>>,
    domain: &str,
    code: Option<&str>,
    mut db: Connection<Db>,
    config: &State<AppConfig>,
    cookies: &CookieJar<'_>,
) -> Result<RespondOrRedirect, FediurlError> {
    let Some(code) = code else {
        return Err(FediurlError::InvalidPath)
    };
    let instance = Instance::from_domain(&mut *db, domain).await?;

    // TODO: Extract method for building a client
    let client = Client::builder()
        .user_agent(format!("Fediurl {}", env!("CARGO_PKG_VERSION"))) // TODO: Move Fediurl into a constant
        .build()?;

    // Use client id, secret, and code to get a token
    let prefix = safe_host(host, &proto, &config);
    let redirect_uri = uri!(prefix, auth(domain = domain, code = _)).to_string();

    let url = instance.url().join("/oauth/token")?;
    let resp = client
        .post(url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &instance.client_id),
            ("client_secret", &instance.client_secret),
            ("redirect_uri", &redirect_uri.to_string()),
            ("scope", SCOPES),
        ])
        .send()
        .await?; // TODO: Add context info to error
    let Ok(token) = json_or_error::<TokenResponse>(resp).await else {
        todo!("render error in flash message");
    };

    // Create the user record.
    let new_user = NewUser {
        instance_id: instance.id,
        access_token: token.access_token,
    };
    let user_id = User::create(&mut *db, new_user).await?; // FIXME: Report nicer error

    // Set login cookie
    let cookie = Cookie::build(FEDIURL_SESSION, user_id.value().to_string())
        .path("/")
        .secure(proto.map_or(false, |proto| &*proto == "https"))
        .http_only(true)
        .max_age(Duration::weeks(1)) // FIXME: Make this a lot longer
        .finish();
    cookies.add_private(cookie);

    Ok(RespondOrRedirect::FlashRedirect(Flash::success(
        Redirect::to(uri!(web::home)),
        "Log in successful",
    )))
}

impl AuthenticatedUser {
    pub fn id(&self) -> i64 {
        self.0.id.value()
    }

    pub fn into_inner(self) -> User {
        self.0
    }
}

impl From<sqlx::Error> for AuthenticatedUserError {
    fn from(err: sqlx::Error) -> Self {
        AuthenticatedUserError::Database(err)
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct ErrorResponse {
    // error: String,
    error_description: String,
}

enum ErrorOrResponse {
    Error(reqwest::Error),
    Response(ErrorResponse),
}

async fn json_or_error<T: DeserializeOwned>(response: Response) -> Result<T, ErrorOrResponse> {
    if response.status().is_success() {
        let app = response.json().await.map_err(ErrorOrResponse::Error)?;
        Ok(app)
    } else {
        error!("Request was unsuccessful ({})", response.status().as_u16());
        // TODO: Distinguish 4xx and 5xx responses
        let err: ErrorResponse = response.json().await.map_err(ErrorOrResponse::Error)?;
        Err(ErrorOrResponse::Response(err))
    }
}
