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

#[derive(sqlx::FromRow, Debug)]
pub struct SavedUser {
    pub id: i64,
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

    pub async fn save_user(&self, user: &DbUser) -> Result<SavedUser, Error> {
        let user_json = serde_json::to_string(user)?;
        let object_id = user.ap_id.inner().to_string();

        //"INSERT INTO users (name, user, object_id) VALUES (?, ?, ?) returning id",
        //
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

    pub async fn find_by_object_id(&self, object_id: &str) -> Result<Option<DbUser>, Error> {
        tracing::debug!("find_by_object_id: {}", object_id);
        let user: Option<SqliteUser> = sqlx::query_as!(
            SqliteUser,
            r#"SELECT id, name, object_id, user AS "user: sqlx::types::Json<DbUser>" FROM users WHERE object_id = ?"#,
            object_id
        )
        .fetch_optional(&self.pool)
        .await?;
        user.map(|u| Ok(u.into())).transpose()
    }

    pub async fn get_followers(&self, user: &DbUser) -> Result<Vec<Follower>, Error> {
        let followers: Vec<Follower> = sqlx::query!(
            r#"SELECT followers.follower_url FROM followers INNER JOIN users ON users.id = followers.user_id WHERE users.name = ?"#,
            user.name
        ).fetch_all(&self.pool).await?.iter().map(|f| Follower::try_from(f.follower_url.clone())).collect::<Result<Vec<Follower>, Error>>()?;

        Ok(followers)
    }

    pub async fn remove_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error> {
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

    pub async fn save_follower(&self, user: &DbUser, follower: &DbUser) -> Result<(), Error> {
        if let Some(_user) = self.read_user(&follower.name).await? {
            // follower user already exists, do nothing
        }
        else {
            // save new user in the database
            self.save_user(follower).await?;
        };

        let follower_url = follower.ap_id.inner().to_string();

        tracing::debug!("save_follower: {} {}", user.name, follower_url);

        sqlx::query!(
            r#"INSERT INTO followers (user_id, follower_id, follower_url) VALUES (
                (SELECT id FROM users WHERE name = ?), 
                (SELECT id FROM users WHERE name = ?)
                , ?)"#,
            user.name,
            follower.name,
            follower_url
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
