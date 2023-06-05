pub mod rewrite;
pub mod session;
mod r#static;

use std::borrow::Cow;
use std::time::Instant;

use rocket::fairing::{self, AdHoc, Fairing, Info, Kind};
use rocket::figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment, Profile,
};
use rocket::http::{ContentType, Status};
use rocket::outcome::IntoOutcome;
use rocket::request::FlashMessage;
use rocket::request::{FromRequest, Outcome};
use rocket::response::content::RawHtml;
use rocket::serde::json::serde_json::json;
use rocket::{Build, Data, Request, Response, Rocket};
use rocket::{Catcher, Route, State};
use rocket_db_pools::{sqlx, Connection, Database};

use crate::config::AppConfig;
use crate::db::Db;
use crate::templates::{Home, Layout, Nil, Privacy, Title};
use crate::web::session::AuthenticatedUser;
use crate::{html, FediurlError};

pub fn rocket() -> Rocket<Build> {
    let figment = Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file("Fediurl.toml").nested())
        .merge(Env::prefixed("FEDIURL_").global())
        .select(Profile::from_env_or("FEDIURL_PROFILE", "default"));

    rocket::custom(figment)
        .attach(stage())
        .attach(RequestTimer(None))
        .attach(AdHoc::config::<AppConfig>())
        .mount("/", routes())
        .mount("/", session::routes())
        .mount("/", rewrite::routes())
        .mount("/", r#static::routes())
        .register("/", catchers())
}

pub fn routes() -> Vec<Route> {
    routes![home, privacy]
}

pub fn catchers() -> Vec<Catcher> {
    catchers![not_found, payload_too_large, internal_server_error]
}

#[get("/")]
pub(crate) async fn home<'f>(
    mut db: Connection<Db>,
    config: &State<AppConfig>,
    flash: Option<FlashMessage<'f>>,
    current_user: Option<AuthenticatedUser>,
) -> Result<RawHtml<String>, FediurlError> {
    let instance = match current_user {
        Some(ref user) => Some(user.instance(&mut *db).await?),
        None => None,
    };
    let page = Layout {
        config: config,
        title: Title::head("Home"),
        flash: flash.as_ref(),
        current_user: current_user.as_ref(),
        head: Nil {},
        body: Home { instance },
    };
    Ok(html(page))
}

#[get("/privacy")]
pub(crate) async fn privacy<'f>(
    config: &State<AppConfig>,
    current_user: Option<AuthenticatedUser>,
    flash: Option<FlashMessage<'f>>,
) -> Result<RawHtml<String>, FediurlError> {
    let page = Layout {
        config: config,
        title: Title::head("Privacy & Security"),
        flash: flash.as_ref(),
        current_user: current_user.as_ref(),
        head: Nil {},
        body: Privacy {},
    };
    Ok(html(page))
}

#[catch(404)]
fn not_found() -> RawHtml<&'static str> {
    const BODY: &str = include_str!("templates/404.html");
    RawHtml(BODY)
}

#[catch(413)]
pub fn payload_too_large<'r>(
    status: Status,
    req: &'r Request<'_>,
) -> (ContentType, Cow<'static, str>) {
    let preferred = req.accept().map(|a| a.preferred());
    if preferred.map_or(false, |a| a.is_json()) {
        let json: Cow<'_, str> = json!({
             "error": {
                "code": 413,
                "reason": "Payload Too Large",
                "description": "The request payload exceeded allowed limits.",
                "limits": req.limits(),
              }
        })
        .to_string()
        .into();

        (ContentType::JSON, json)
    } else {
        const BODY: &str = include_str!("templates/413.html");
        (ContentType::HTML, BODY.into())
    }
}

#[catch(500)]
fn internal_server_error() -> RawHtml<&'static str> {
    const BODY: &str = include_str!("templates/500.html");
    RawHtml(BODY)
}

pub fn stage() -> AdHoc {
    // TODO: Set SQLite options:
    // synchronous = NORMAL, should be enough when WAL is used. Rocket/SQLx defaults to FULL
    // https://api.rocket.rs/v0.5-rc/rocket_db_pools/index.html#driver-defaults

    AdHoc::on_ignite("SQLx Stage", |rocket| async {
        rocket
            .attach(Db::init())
            .attach(AdHoc::try_on_ignite("SQLx Migrations", run_migrations))
            .attach(AdHoc::try_on_ignite("Load config", init_sentry))
    })
}

async fn run_migrations(rocket: Rocket<Build>) -> fairing::Result {
    match Db::fetch(&rocket) {
        Some(db) => match sqlx::migrate!("./migrations").run(&**db).await {
            Ok(_) => Ok(rocket),
            Err(e) => {
                error!("Failed to initialize SQLx database: {}", e);
                Err(rocket)
            }
        },
        None => Err(rocket),
    }
}

pub async fn init_sentry(mut rocket: Rocket<Build>) -> fairing::Result {
    let config = rocket.state::<AppConfig>().unwrap();

    if let Some(dsn) = config.sentry_dsn.as_deref() {
        let guard = sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                attach_stacktrace: true,
                ..Default::default()
            },
        ));
        info!("Sentry initialised");
        rocket = rocket.manage(guard)
    } else {
        info!("No Sentry DSN")
    }

    Ok(rocket)
}

#[derive(Copy, Clone)]
struct RequestTimer(Option<Instant>);

#[rocket::async_trait]
impl Fairing for RequestTimer {
    fn info(&self) -> Info {
        Info {
            name: "Request timer",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        request.local_cache(|| RequestTimer(Some(Instant::now())));
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let start_time = request.local_cache(|| RequestTimer(None));
        if let Some(Some(duration)) = start_time
            .0
            .map(|st| Instant::now().checked_duration_since(st))
        {
            let us = duration.as_micros();
            if us < 1000 {
                response.set_raw_header("X-Response-Time", format!("{} us", us));
            } else {
                let ms = us / 1000;
                response.set_raw_header("X-Response-Time", format!("{} ms", ms));
            }
        }
    }
}

/// Request guard used to retrieve the start time of a request.
#[derive(Copy, Clone)]
pub struct StartTime(pub Instant);

// Allows a route to access the time a request was initiated.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for StartTime {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match *request.local_cache(|| RequestTimer(None)) {
            RequestTimer(Some(time)) => Outcome::Success(StartTime(time)),
            RequestTimer(None) => Outcome::Failure((Status::InternalServerError, ())),
        }
    }
}

/// Request guard used to retrieve the start time of a request.
#[derive(Copy, Clone)]
pub struct XForwardedProto<'r>(pub &'r str);

// Allows a route to access the time a request was initiated.
#[rocket::async_trait]
impl<'r> FromRequest<'r> for XForwardedProto<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        request
            .headers()
            .get_one("X-Forwarded-Proto")
            .map(|proto| XForwardedProto(proto))
            .or_forward(())
        // match *request.local_cache(|| RequestTimer(None)) {
        //     RequestTimer(Some(time)) => Outcome::Success(StartTime(time)),
        //     RequestTimer(None) => Outcome::Failure((Status::InternalServerError, ())),
        // }
    }
}

impl std::ops::Deref for XForwardedProto<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}
