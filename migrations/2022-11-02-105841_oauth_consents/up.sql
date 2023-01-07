CREATE TABLE oauth_consents (
    id UUID PRIMARY KEY,
    app_id UUID REFERENCES apps(id),
    user_id VARCHAR NOT NULL,
    time TIMESTAMP NOT NULL
);

CREATE TABLE oauth_consent_scopes (
    consent_id UUID REFERENCES oauth_consents(id) ON DELETE CASCADE,
    scope VARCHAR NOT NULL,
    PRIMARY KEY (consent_id, scope)
);

CREATE INDEX oauth_consent_scopes_consent_id ON oauth_consent_scopes(consent_id);