// Database utilities for the blog backend
// This file contains shared database functionality
#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug)]
pub struct SavedBlogPost {
    pub id: i64,
}
