-- Migration to replace IP addresses with their hashes for privacy
-- This migration converts existing INET columns to VARCHAR hashes

-- Enable pgcrypto extension for digest function
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Step 1: Add new columns for hashed IPs
ALTER TABLE blog_post_likes
ADD COLUMN user_ip_hash VARCHAR(64),
ADD COLUMN cf_connecting_ip_hash VARCHAR(64);

-- Step 2: Migrate existing data by hashing IPs
-- Note: This uses PostgreSQL's digest function with SHA256
UPDATE blog_post_likes
SET user_ip_hash = encode(digest(host(user_ip)::text, 'sha256'), 'hex'),
    cf_connecting_ip_hash = CASE
        WHEN cf_connecting_ip IS NOT NULL
        THEN encode(digest(host(cf_connecting_ip)::text, 'sha256'), 'hex')
        ELSE NULL
    END;

-- Step 3: Make user_ip_hash NOT NULL since user_ip was NOT NULL
ALTER TABLE blog_post_likes
ALTER COLUMN user_ip_hash SET NOT NULL;

-- Step 4: Drop the old IP columns
ALTER TABLE blog_post_likes
DROP COLUMN user_ip,
DROP COLUMN cf_connecting_ip;

-- Step 5: Drop old index that referenced user_ip
DROP INDEX IF EXISTS idx_blog_post_likes_ip_time;

-- Step 6: Create new index on hashed IP
CREATE INDEX idx_blog_post_likes_ip_hash_time ON blog_post_likes(user_ip_hash, liked_at);

-- Step 7: Recreate unique constraint with hashed IP
ALTER TABLE blog_post_likes
DROP CONSTRAINT IF EXISTS blog_post_likes_post_slug_user_ip_hour_bucket_key;

ALTER TABLE blog_post_likes
ADD CONSTRAINT blog_post_likes_post_slug_user_ip_hash_hour_bucket_key
UNIQUE(post_slug, user_ip_hash, hour_bucket);
