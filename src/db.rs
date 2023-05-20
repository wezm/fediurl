use rocket_db_pools::{sqlx, Database};

#[derive(Database)]
#[database("fediurl_db")]
pub struct Db(pub sqlx::SqlitePool);
