use sqlx::SqliteConnection;
use time::OffsetDateTime;
use url::Url;

#[derive(sqlx::Type, Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[sqlx(transparent)]
pub struct InstanceId(i64);

#[derive(Debug)]
pub struct Instance {
    pub id: InstanceId,
    pub domain: String,
    pub client_id: String,
    pub client_secret: String,
    pub banned_until: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

// TODO: These probably don't need to be owned strings
#[derive(Debug)]
pub struct NewInstance {
    pub domain: String,
    pub client_id: String,
    pub client_secret: String,
}

macro_rules! instance_query {
    ($field:literal, $arg:tt) => {
        sqlx::query_as!(
            Instance,
            r#"SELECT
                id as "id: InstanceId",
                domain,
                client_id,
                client_secret,
                banned_until as "banned_until: OffsetDateTime",
                created_at as "created_at: OffsetDateTime",
                updated_at as "updated_at: OffsetDateTime"
            FROM instances
            WHERE "#
                + $field
                + " = ?",
            $arg,
        )
    };
}

impl Instance {
    /// Inserts a new user into the database and returns its id
    pub async fn create(
        db: &mut SqliteConnection,
        instance: NewInstance,
    ) -> Result<InstanceId, sqlx::Error> {
        let NewInstance {
            domain,
            client_id,
            client_secret,
        } = instance;

        let res = sqlx::query!(
            "INSERT INTO instances (domain, client_id, client_secret) VALUES (?, ?, ?)",
            domain,
            client_id,
            client_secret
        )
        .execute(db)
        .await?;

        Ok(InstanceId(res.last_insert_rowid()))
    }

    pub async fn from_id(
        db: &mut SqliteConnection,
        id: InstanceId,
    ) -> Result<Instance, sqlx::Error> {
        instance_query!("id", id).fetch_one(db).await
    }

    pub async fn from_domain(
        db: &mut SqliteConnection,
        domain: &str,
    ) -> Result<Instance, sqlx::Error> {
        instance_query!("domain", domain).fetch_one(db).await
    }

    pub async fn from_domain_optional(
        db: &mut SqliteConnection,
        domain: &str,
    ) -> Result<Option<Instance>, sqlx::Error> {
        instance_query!("domain", domain).fetch_optional(db).await
    }

    pub(crate) fn url(&self) -> Url {
        format!("https://{}", self.domain).parse().unwrap()
    }
}
