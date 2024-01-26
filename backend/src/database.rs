use std::sync::Arc;

use crate::{
    objects::person::{DbUser, Follower, SqliteUser},
    Error, LOCAL_USER_NAME,
};
use anyhow::anyhow;
use async_trait::async_trait;

/// Our "database" which contains all known users (local and federated)
#[derive(Clone)]
pub struct SqlDatabase {
    pub pool: sqlx::SqlitePool,
}

#[derive(sqlx::FromRow, Debug)]
pub struct SavedUser {
    pub id: i64,
}

pub type Repository = Arc<dyn Db + Send + Sync>;

#[async_trait]
pub trait Db {
    async fn blog_user(&self) -> Result<DbUser, Error>;

    async fn user_by_name(&self, name: &str) -> Result<Option<DbUser>, Error>;

    async fn user_by_object_id(&self, object_id: &str) -> Result<Option<DbUser>, Error>;

    async fn user_followers(&self, user: &DbUser) -> Result<Vec<Follower>, Error>;

    async fn save_user(&self, user: &DbUser) -> Result<SavedUser, Error>;

    async fn remove_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error>;

    async fn add_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error>;
}

#[async_trait]
impl Db for SqlDatabase {
    async fn blog_user(&self) -> Result<DbUser, Error> {
        let user = self.user_by_name(LOCAL_USER_NAME).await?;
        match user {
            Some(user) => Ok(user),
            None => Err(anyhow!("Local user not found").into()),
        }
    }

    async fn user_by_name(&self, name: &str) -> Result<Option<DbUser>, Error> {
        let user: Option<SqliteUser> = sqlx::query_as!(
            SqliteUser,
            r#"SELECT id, name, object_id, user AS "user: sqlx::types::Json<DbUser>" FROM users WHERE name = ?"#,
            name
        )
        .fetch_optional(&self.pool)
        .await?;
        user.map(|u| Ok(u.into())).transpose()
    }

    async fn save_user(&self, user: &DbUser) -> Result<SavedUser, Error> {
        let user_json = serde_json::to_string(user)?;
        let object_id = user.ap_id.inner().to_string();

        let id: SavedUser = sqlx::query_as!(
            SavedUser,
            r#"INSERT INTO users (name, user, object_id) VALUES (?, ?, ?) returning id"#,
            user.name,
            user_json,
            object_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(id)
    }

    async fn user_by_object_id(&self, object_id: &str) -> Result<Option<DbUser>, Error> {
        let user: Option<SqliteUser> = sqlx::query_as!(
            SqliteUser,
            r#"SELECT id, name, object_id, user AS "user: sqlx::types::Json<DbUser>" FROM users WHERE object_id = ?"#,
            object_id
        )
        .fetch_optional(&self.pool)
        .await?;
        user.map(|u| Ok(u.into())).transpose()
    }

    async fn user_followers(&self, user: &DbUser) -> Result<Vec<Follower>, Error> {
        let followers: Vec<Follower> = sqlx::query!(
            r#"SELECT followers.follower_url FROM followers INNER JOIN users ON users.id = followers.user_id WHERE users.name = ?"#,
            user.name
        ).fetch_all(&self.pool).await?.iter().map(|f| Follower::try_from(f.follower_url.clone())).collect::<Result<Vec<Follower>, Error>>()?;

        Ok(followers)
    }

    async fn remove_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error> {
        let follower_url = follower.ap_id.inner().to_string();

        tracing::debug!("remove_follower: {} {}", user.name, follower_url);

        sqlx::query!(
            r#"DELETE FROM followers WHERE user_id = (SELECT id FROM users WHERE name = ?) AND follower_id = (SELECT id FROM users WHERE name = ?)"#,
            user.name,
            follower.name
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_user_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error> {
        if let Some(_user) = self.user_by_name(&follower.name).await? {
            // follower user already exists no need to save him
        } else {
            // save new user - follower in the database
            self.save_user(follower).await?;
        };

        let follower_url = follower.ap_id.inner().to_string();

        sqlx::query!(
            r#"INSERT INTO followers (user_id, follower_id, follower_url) VALUES (
                (SELECT id FROM users WHERE name = ?), 
                (SELECT id FROM users WHERE name = ?)
                , ?) ON CONFLICT (user_id, follower_id) DO NOTHING"#,
            user.name,
            follower.name,
            follower_url
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
