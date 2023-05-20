use rocket::form;
use rocket::form::Error;
use rocket::http::uri::{Absolute, Host};

use crate::form::NonEmptyString;

pub fn domain<'v>(value: &str) -> form::Result<'v, Host> {
    match Host::parse(value) {
        Ok(host) => Ok(host),
        Err(err) => Err(Error::validation(format!("instance is not valid: {}", err)).into()),
    }
}

pub fn uri_absolute<'v>(value: &NonEmptyString<'v>) -> form::Result<'v, ()> {
    let uri = Absolute::parse(&value).map_err(|_err| Error::validation("is not a valid URL"))?;

    if uri.scheme() != "http" && uri.scheme() != "https" {
        return Err(Error::validation("scheme must be http or https").into());
    }

    match uri.authority() {
        Some(authority) if authority.host().is_empty() => {
            Err(Error::validation("host is missing").into())
        }
        Some(_) => Ok(()),
        None => Err(Error::validation("is incomplete").into()),
    }
}
