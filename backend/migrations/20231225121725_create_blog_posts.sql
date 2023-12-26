CREATE TABLE blog_posts (
    slug TEXT PRIMARY KEY NOT NULL, -- Slug is used as a primary key, so it must be unique
    title TEXT NOT NULL,
    description TEXT,
    date TEXT NOT NULL, -- SQLite does not have a dedicated DateTime type, so TEXT is used
    featured_image TEXT, -- This is optional, so it can be NULL
    tags TEXT, -- comma-separated list of tags
    url TEXT NOT NULL -- URL as TEXT
);

-- slug is a foreign key and primary key in blog_posts_published
CREATE TABLE blog_posts_published (
    slug TEXT PRIMARY KEY NOT NULL,
    FOREIGN KEY(slug) REFERENCES blog_posts(slug)
);

