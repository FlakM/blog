CREATE TABLE fediverse_received_activities (
    activity_id VARCHAR PRIMARY KEY,
    actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    activity_type VARCHAR NOT NULL,
    received_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE fediverse_reactions (
    activity_id VARCHAR PRIMARY KEY,
    post_slug VARCHAR NOT NULL REFERENCES blog_posts(slug) ON DELETE CASCADE,
    actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    kind VARCHAR NOT NULL CHECK (kind IN ('Like', 'Announce')),
    object_id VARCHAR NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE (post_slug, actor_id, kind)
);

CREATE INDEX fediverse_reactions_post_slug ON fediverse_reactions(post_slug);

CREATE TABLE fediverse_undone_activities (
    activity_id VARCHAR PRIMARY KEY,
    actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    undone_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE TABLE fediverse_replies (
    object_id VARCHAR PRIMARY KEY,
    create_activity_id VARCHAR NOT NULL UNIQUE,
    post_slug VARCHAR NOT NULL REFERENCES blog_posts(slug) ON DELETE CASCADE,
    actor_id VARCHAR NOT NULL REFERENCES fediverse_actors(ap_id) ON DELETE CASCADE,
    url VARCHAR NOT NULL,
    content TEXT NOT NULL,
    published_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE,
    deleted_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX fediverse_replies_post_slug ON fediverse_replies(post_slug);
