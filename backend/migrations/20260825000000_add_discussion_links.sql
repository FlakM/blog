CREATE TABLE blog_post_discussion_links (
    post_slug VARCHAR NOT NULL REFERENCES blog_posts(slug) ON DELETE CASCADE,
    source VARCHAR NOT NULL,
    label VARCHAR NOT NULL,
    url VARCHAR NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (post_slug, source, url),
    CONSTRAINT discussion_source_not_empty CHECK (btrim(source) <> ''),
    CONSTRAINT discussion_label_not_empty CHECK (btrim(label) <> ''),
    CONSTRAINT discussion_url_is_web_url CHECK (url ~ '^https?://')
);
