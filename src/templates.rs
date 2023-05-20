pub mod form;
mod home;
mod layout;
pub mod session;

use rocket::request::FlashMessage;

pub use home::Home;
pub use layout::{Layout, Nil, Title};

markup::define! {
    Flash<'a, 'f>(flash: Option<&'a FlashMessage<'f>>) {
        @if let Some(flash) = flash {
            @if flash.kind() == "error" {
                .flash."flash-error" { @flash.message() }
            }
            else if flash.kind() == "warning" {
                .flash."flash-warning" { @flash.message() }
            }
            else if flash.kind() == "success" {
                .flash."flash-success" { @flash.message() }
            }
            else {
                .flash { @flash.message() }
            }
        }
    }
}
