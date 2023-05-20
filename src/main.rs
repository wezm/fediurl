#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    fediurl::web::rocket()
}
