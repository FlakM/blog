CREATE TABLE followers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    follower_id INTEGER NOT NULL,
    follower_url TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users (id),
    FOREIGN KEY (follower_id) REFERENCES users (id)
);

CREATE INDEX index_followers_on_user_id_and_follower_id ON followers (user_id, follower_id);


-- write select to find all followers of a user

