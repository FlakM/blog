CREATE TABLE fediverse_actors (
    ap_id VARCHAR PRIMARY KEY,
    username VARCHAR NOT NULL,
    inbox VARCHAR NOT NULL,
    shared_inbox VARCHAR,
    public_key TEXT NOT NULL,
    private_key TEXT,
    last_refreshed_at TIMESTAMP WITH TIME ZONE NOT NULL,
    local BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE UNIQUE INDEX fediverse_local_actor_username
    ON fediverse_actors (username)
    WHERE local;

CREATE TABLE fediverse_followers (
    local_actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    follower_actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    followed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (local_actor_id, follower_actor_id)
);

CREATE TABLE fediverse_published_posts (
    slug VARCHAR PRIMARY KEY REFERENCES blog_posts(slug) ON DELETE CASCADE,
    published_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);
