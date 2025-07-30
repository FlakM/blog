-- Create table for storing blog post likes
CREATE TABLE blog_post_likes (
    id SERIAL PRIMARY KEY,
    post_slug VARCHAR NOT NULL,
    user_ip INET NOT NULL,
    user_agent TEXT,
    cf_country VARCHAR(2), -- Cloudflare country header (ISO country code)
    cf_connecting_ip INET, -- Cloudflare connecting IP header
    liked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    hour_bucket VARCHAR(13) NOT NULL, -- Format: 'YYYY-MM-DD HH' for rate limiting
    FOREIGN KEY(post_slug) REFERENCES blog_posts(slug),
    UNIQUE(post_slug, user_ip, hour_bucket) -- One like per hour per IP per post
);

-- Create index for faster lookups
CREATE INDEX idx_blog_post_likes_post_slug ON blog_post_likes(post_slug);
CREATE INDEX idx_blog_post_likes_ip_time ON blog_post_likes(user_ip, liked_at);