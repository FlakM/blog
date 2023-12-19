use crate::{
    objects::person::{DbUser, Follower, SqliteUser},
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
        let object_id = user.ap_id.inner().to_string();
        sqlx::query!(
            "INSERT INTO users (name, user, object_id) VALUES (?, ?, ?)",
            user.name,
            user_json,
            object_id
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

    pub async fn get_followers(&self, user: &DbUser) -> Result<Vec<Follower>, Error> {
        let followers: Vec<Follower> = sqlx::query!(
            r#"SELECT followers.follower_url FROM followers INNER JOIN users ON users.id = followers.user_id WHERE users.name = ?"#,
            user.name
        ).fetch_all(&self.pool).await?.iter().map(|f| Follower::try_from(f.follower_url.clone())).collect::<Result<Vec<Follower>, Error>>()?;

        Ok(followers)
    }

    pub async fn save_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error> {
        // todo: check if follower already exists
        self.save_user(follower).await?;
        let follower_url = follower.ap_id.inner().to_string();
        sqlx::query!(
            r#"INSERT INTO followers (user_id, follower_url) VALUES ((SELECT id FROM users WHERE name = ?), ?)"#,
            user.name,
            follower_url
        ).execute(&self.pool).await?;

        Ok(())
    }
}
