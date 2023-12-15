use crate::{
    objects::person::{DbUser, SqliteUser},
    Error, LOCAL_USER_NAME,
};
use anyhow::anyhow;

/// Our "database" which contains all known users (local and federated)
#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::SqlitePool,
}

impl Database {
    pub async fn local_user(&self) -> Result<DbUser, Error> {
        let user = self.read_user(LOCAL_USER_NAME).await?;
        match user {
            Some(user) => Ok(user),
            None => Err(anyhow!("Local user not found").into()),
        }
    }

    pub async fn read_user(&self, name: &str) -> Result<Option<DbUser>, Error> {
        let user: Option<SqliteUser> = sqlx::query_as!(
            SqliteUser,
            r#"SELECT id, name, object_id, user AS "user: sqlx::types::Json<DbUser>" FROM users WHERE name = ?"#,
            name
        )
        .fetch_optional(&self.pool)
        .await?;
        user.map(|u| Ok(u.into())).transpose()
    }

    pub async fn save_user(&self, user: &DbUser) -> Result<(), Error> {
        let user_json = serde_json::to_string(user)?;
        sqlx::query!(
            "INSERT INTO users (name, user) VALUES (?, ?)",
            user.name,
            user_json
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_object_id(&self, object_id: &str) -> Result<DbUser, Error> {
        let user: Option<SqliteUser> = sqlx::query_as!(
            SqliteUser,
            r#"SELECT id, name, object_id, user AS "user: sqlx::types::Json<DbUser>" FROM users WHERE object_id = ?"#,
            object_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match user {
            Some(user) => Ok(user.into()),
            None => Err(anyhow!("User not found").into()),
        }
    }
}
