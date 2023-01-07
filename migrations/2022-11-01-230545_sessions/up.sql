CREATE TABLE session
(
    id            UUID PRIMARY KEY,
    access_token  VARCHAR NOT NULL,
    expires_at    TIMESTAMP,
    refresh_token VARCHAR,
    claims        VARCHAR NOT NULL
);