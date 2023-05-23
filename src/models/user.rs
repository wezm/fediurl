use std::fmt;
use std::str::FromStr;

use sqlx::SqliteConnection;
use time::OffsetDateTime;

use crate::models::instance::{Instance, InstanceId};

#[derive(sqlx::Type, Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[sqlx(transparent)]
pub struct UserId(i64);

#[derive(Debug)]
pub struct User {
    pub id: UserId,
    pub instance_id: InstanceId,
    pub access_token: String,
    pub banned_until: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// TODO: These probably don't need to be owned strings
#[derive(Debug)]
pub struct NewUser {
    pub instance_id: InstanceId,
    pub access_token: String,
}

impl UserId {
    pub fn value(&self) -> i64 {
        self.0
    }
}

impl From<i64> for UserId {
    fn from(id: i64) -> Self {
        UserId(id)
    }
}

impl FromStr for UserId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(UserId)
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! user_query {
    ($field:literal, $arg:tt) => {
        sqlx::query_as!(
            User,
            r#"SELECT
                id as "id: UserId",
                instance_id as "instance_id: InstanceId",
                access_token,
                banned_until as "banned_until: OffsetDateTime",
                created_at as "created_at: OffsetDateTime",
                updated_at as "updated_at: OffsetDateTime"
            FROM users
            WHERE "#
                + $field
                + " = ?",
            $arg,
        )
    };
}

impl User {
    /// Inserts a new user into the database and returns its id
    pub async fn create(db: &mut SqliteConnection, user: NewUser) -> Result<UserId, sqlx::Error> {
        let NewUser {
            instance_id,
            access_token,
        } = user;

        let res = sqlx::query!(
            "INSERT INTO users (instance_id, access_token) VALUES (?, ?)",
            instance_id,
            access_token
        )
        .execute(db)
        .await?;

        Ok(UserId(res.last_insert_rowid()))
    }

    pub async fn from_id(db: &mut SqliteConnection, user_id: UserId) -> Result<User, sqlx::Error> {
        user_query!("id", user_id).fetch_one(db).await
    }

    pub async fn instance(&self, db: &mut SqliteConnection) -> Result<Instance, sqlx::Error> {
        Instance::from_id(&mut *db, self.instance_id).await
    }
}
