ALTER TABLE fediverse_actors
ADD COLUMN display_name VARCHAR,
ADD COLUMN profile_url VARCHAR,
ADD COLUMN avatar_url VARCHAR;

UPDATE fediverse_actors
SET last_refreshed_at = TO_TIMESTAMP(0)
WHERE NOT local;
