use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

use rocket::http::ContentType;
use rocket::Route;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "public"]
pub struct Asset;

pub fn routes() -> Vec<Route> {
    routes![public]
}

#[get("/public/<file..>")]
fn public(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = file.to_str()?;
    let asset = Asset::get(filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}
