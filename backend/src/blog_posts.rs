use std::{fs::File, io::BufReader, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteRow, Error, FromRow, Row};
use url::Url;

use crate::{objects::{person::DbUser, post::DbPost}, utils::generate_object_id};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlogPost {
    /// Human-readable title - it should be a short sentence - single line
    title: String,
    /// The identifier for the post a slug from frontmatter or the filename
    /// if no slug is provided.
    ///
    /// This is used as main identifier in blog_posts database table
    pub slug: String,
    /// A short description of the post - it should be a single paragraph or two
    description: String,
    /// The date the post was created
    date: DateTime<Utc>,
    /// An image to use as the featured image for the blog post
    featured_image: Option<String>,
    /// List of tags for the post it will be mapped to mastodon tags
    tags: Option<Vec<String>>,
    /// The URL of the post itself
    url: Url,
}

impl FromRow<'_, SqliteRow> for BlogPost {
    fn from_row(row: &SqliteRow) -> Result<Self, Error> {
        let title: String = row.try_get("title")?;
        let slug: String = row.try_get("slug")?;
        let description: String = row.try_get("description")?;
        let date_str: String = row.try_get("date")?;
        let featured_image: Option<String> = row.try_get("featured_image")?;
        let tags_str: Option<String> = row.try_get("tags")?;
        let url_str: String = row.try_get("url")?;

        // Parse the date
        let date =
            DateTime::parse_from_rfc3339(&date_str).map_err(|e| Error::Decode(Box::new(e)))?;

        // Parse the URL
        let url = Url::parse(&url_str).map_err(|e| Error::Decode(Box::new(e)))?;

        // Parse the tags if they exist
        let tags = tags_str.map(|s| s.split(',').map(String::from).collect());

        Ok(BlogPost {
            title,
            slug,
            description,
            date: date.with_timezone(&Utc),
            featured_image,
            tags,
            url,
        })
    }
}

type BlogPosts = Vec<BlogPost>;

impl BlogPost {
    /// Read and deserialize the blog posts from the filesystem file
    pub fn load_new_posts(path: impl AsRef<Path>) -> Result<BlogPosts, crate::Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let blog_posts = serde_json::from_reader(reader)?;

        Ok(blog_posts)
    }

    pub fn to_post(&self, local_user: &DbUser, domain: &str) -> Result<DbPost, crate::Error> {
        let content = format!(
            r#"<p>{}</p><p>{}</p><p><a href="{}">Read more</a></p>"#,
            self.title, self.description, self.url
        );
        
        Ok(DbPost {
            text: content,
            ap_id: generate_object_id(domain)?.into(),
            creator: local_user.ap_id.clone(),
            local: true,
        })
    }
}

pub struct BlogRepository {
    pub db: sqlx::SqlitePool,
}

impl BlogRepository {
    /// Create a new blog post entry in the database - called when a new blog post is present in
    /// json file from static site generator
    pub async fn new_blog_entry(&self, blog_post: &BlogPost) -> Result<(), Error> {
        let tags_str = blog_post.tags.clone().map(|tags| tags.join(","));
        sqlx::query(
            "INSERT INTO blog_posts (title, slug, description, date, featured_image, tags, url) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(slug) DO UPDATE SET title = excluded.title, description = excluded.description, date = excluded.date, featured_image = excluded.featured_image, tags = excluded.tags, url = excluded.url",
        )
        .bind(&blog_post.title)
        .bind(&blog_post.slug)
        .bind(&blog_post.description)
        .bind(&blog_post.date.to_rfc3339())
        .bind(&blog_post.featured_image)
        .bind(&tags_str)
        .bind(&blog_post.url.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get all blog posts that have not been published yet
    /// post is published when it is posted to mastodon
    /// and the record is present in blog_posts_published
    pub async fn get_unpublished_blog_posts(&self) -> Result<Vec<BlogPost>, Error> {
        let blog_posts = sqlx::query_as::<_, BlogPost>(
            "SELECT * FROM blog_posts WHERE slug NOT IN (SELECT slug FROM blog_posts_published)",
        )
        .fetch_all(&self.db)
        .await?;

        Ok(blog_posts)
    }
}
