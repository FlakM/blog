ALTER TABLE fediverse_replies
ADD COLUMN in_reply_to VARCHAR;

CREATE INDEX fediverse_replies_in_reply_to ON fediverse_replies(in_reply_to);

UPDATE blog_post_discussion_links
SET label = 'Reply on Hachyderm'
WHERE source = 'mastodon';
