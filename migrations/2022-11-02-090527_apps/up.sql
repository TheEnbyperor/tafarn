CREATE TABLE apps (
      id            UUID PRIMARY KEY,
      name          TEXT NOT NULL,
      website       TEXT,
      redirect_uri  TEXT NOT NULL,
      client_secret VARCHAR NOT NULL
);

CREATE TABLE app_scopes (
    app_id  UUID REFERENCES apps(id) ON DELETE CASCADE,
    scope   TEXT NOT NULL,
    PRIMARY KEY (app_id, scope)
);

CREATE INDEX app_scopes_app_id ON app_scopes (app_id);