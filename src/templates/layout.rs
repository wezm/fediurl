use std::borrow::Cow;
use std::ops::RangeInclusive;

use rocket::request::FlashMessage;
use time::OffsetDateTime;

use super::Flash;
use crate::config::{self, AppConfig};
use crate::templates::session::Logout;
use crate::web::session::AuthenticatedUser;

pub struct Title<'a> {
    title: Cow<'a, str>,
    visibility: TitleVisibility,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum TitleVisibility {
    Head,
    HeadAndBody,
}

impl<'a> Title<'a> {
    pub fn head_and_body<S: Into<Cow<'a, str>>>(title: S) -> Title<'a> {
        Title {
            title: title.into(),
            visibility: TitleVisibility::HeadAndBody,
        }
    }

    pub fn head<S: Into<Cow<'a, str>>>(title: S) -> Title<'a> {
        Title {
            title: title.into(),
            visibility: TitleVisibility::Head,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn visibility(&self) -> TitleVisibility {
        self.visibility
    }
}

markup::define! {
    Layout<'a, Head: markup::Render, Body: markup::Render>(title: Title<'a>, head: Head, body: Body, config: &'a AppConfig, flash: Option<&'a FlashMessage<'a>>, current_user: Option<&'a AuthenticatedUser>) {
        @markup::doctype()

        html[lang="en"] {
            head {
                meta[charset="utf-8"];
                meta[name="viewport", content="width=device-width, initial-scale=1"];
                title { @crate::NAME " - " @title.title() }
                link[rel="stylesheet", href="/public/css/theme.css", type="text/css", charset="utf-8"];
                link[rel="icon", href=r#"data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">↩️</text></svg>"#];
                @head
            }
            body {
                .wrapper {
                    header {
                        h1."pull-left" { a[href = uri!(crate::web::home).to_string()] { @crate::NAME } }
                        nav."text-right" {
                            ul."list-inline" {
                                li {
                                    @if let Some(_user) = current_user {
                                        @Logout {}
                                    }
                                }
                            }
                        }
                    }

                    main {
                        @if title.visibility() == TitleVisibility::HeadAndBody {
                            h2 { @title.title() }
                        }
                        @Flash { flash: *flash }
                        @body
                    }

                    footer."text-center" {
                        .socials {
                            a[href = uri!(crate::web::home).to_string()] { "Home" } " "
                            a[href = uri!(crate::web::privacy).to_string()] { "Privacy & Security" } " "
                            // a[href = uri!(crate::web::home).to_string()] { "Acknowledgements" }
                            " • "
                            a[href="https://decentralised.social/wezm"] { "Fediverse" } " "
                            a[href="https://github.com/wezm"] { "GitHub" } " "
                            a[href="https://github.com/sponsors/wezm"] { "Support My Work" }
                        }
                        .copyright {
                          "Copyright © "
                          @let years = copyright_years();
                          @if years.start() == years.end() {
                              @years.start()
                          }
                          else {
                              @years.start() @markup::raw("&ndash;") @years.end()
                          }
                          " " a[href="https://www.wezm.net/"] { "Wesley Moore" }
                          " — " @crate::NAME " is "
                          a[href="https://github.com/wezm/fediurl"] { "open-source" } "."
                          " (" { config::git_revision() } ")"
                        }
                    }
                }
            }
        }
    }

    // An empty renderer for pages that don't have extra head content
    Nil() {}
}

fn copyright_years() -> RangeInclusive<u16> {
    // TODO: Probably don't need to get the year on every render
    // perhaps it can be cached
    2023..=OffsetDateTime::now_utc().year() as u16
}
