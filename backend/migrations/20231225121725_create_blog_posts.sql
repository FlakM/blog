CREATE TABLE blog_posts (
    slug VARCHAR PRIMARY KEY NOT NULL, -- Slug is used as a primary key, so it must be unique
    title VARCHAR NOT NULL,
    description TEXT,
    date TIMESTAMP WITH TIME ZONE NOT NULL, -- PostgreSQL has proper datetime support
    featured_image VARCHAR, -- This is optional, so it can be NULL
    tags TEXT, -- comma-separated list of tags
    url VARCHAR NOT NULL -- URL as TEXT
);


