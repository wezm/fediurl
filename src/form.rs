use std::ops::Deref;

use join_to_string::join;
use rocket::form::validate::Len;
use rocket::form::Context;
use rocket::form::{self, FromFormField, ValueField};

use crate::string_ext::StringExt;

pub mod validate;

#[derive(Debug, Clone, sqlx::Type)]
#[sqlx(transparent)]
pub struct NonEmptyString<'a>(&'a str);

pub trait ContextExt {
    // Retrieve the value for a field or empty string
    fn value_for(&self, name: &str) -> &str;

    // Returns a comma separated list of fields with errors
    //
    // The fields names are humanised.
    fn fields_with_errors(&self) -> String;
}

impl ContextExt for Context<'_> {
    fn value_for(&self, name: &str) -> &str {
        self.field_value(name).unwrap_or_default()
    }

    fn fields_with_errors(&self) -> String {
        let errors = self
            .errors()
            .flat_map(|err| err.name.as_ref().map(|name| name.to_string().humanise()));
        join(errors).separator(", ").to_string()
    }
}

impl<'a> Deref for NonEmptyString<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r> FromFormField<'r> for NonEmptyString<'r> {
    fn from_value(field: ValueField<'r>) -> form::Result<'r, Self> {
        let trimmed = field.value.trim();
        if trimmed.is_empty() {
            Err(form::Error::validation("cannot be blank").into())
        } else {
            Ok(NonEmptyString(trimmed))
        }
    }
}

impl<'v> Len<usize> for NonEmptyString<'v> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn len_into_u64(len: usize) -> u64 {
        len as u64
    }

    fn zero_len() -> usize {
        0
    }
}
