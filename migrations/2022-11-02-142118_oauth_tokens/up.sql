CREATE TABLE oauth_codes
(
    id           UUID PRIMARY KEY          NOT NULL,
    time         TIMESTAMP                 NOT NULL,
    redirect_uri TEXT                      NOT NULL,
    client_id    UUID REFERENCES apps (id) ON DELETE CASCADE NOT NULL,
    user_id      TEXT                      NOT NULL
);

CREATE TABLE oauth_code_scopes
(
    code_id UUID REFERENCES oauth_codes (id) ON DELETE CASCADE NOT NULL,
    scope   VARCHAR NOT NULL,
    PRIMARY KEY (code_id, scope)
);

CREATE INDEX oauth_code_scopes_code_id ON oauth_code_scopes (code_id);

CREATE TABLE oauth_token
(
    id            UUID PRIMARY KEY          NOT NULL,
    time          TIMESTAMP                 NOT NULL,
    client_id     UUID REFERENCES apps (id) ON DELETE CASCADE NOT NULL,
    user_id       TEXT                      NOT NULL,
    revoked       BOOLEAN                   NOT NULL
);

CREATE TABLE oauth_token_scopes
(
    token_id UUID REFERENCES oauth_token (id) ON DELETE CASCADE NOT NULL,
    scope    VARCHAR NOT NULL,
    PRIMARY KEY (token_id, scope)
);

CREATE INDEX oauth_token_scopes ON oauth_token_scopes (token_id);