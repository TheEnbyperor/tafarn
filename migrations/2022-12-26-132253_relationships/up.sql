CREATE TABLE following (
 id UUID PRIMARY KEY NOT NULL,
 follower UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 followee UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
 created_at TIMESTAMP NOT NULL DEFAULT NOW(),
 UNIQUE (follower, followee)
);

CREATE INDEX following_follower_followee ON following(follower, followee);