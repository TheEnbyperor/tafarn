CREATE TABLE accounts (
    id UUID PRIMARY KEY NOT NULL,
    actor TEXT NULL,
    username VARCHAR(255) NOT NULL,
    display_name TEXT NOT NULL,
    bio TEXT NOT NULL,
    locked BOOLEAN NOT NULL,
    bot BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    avatar TEXT NULL,
    header TEXT NULL,
    default_sensitive BOOL NULL,
    default_language CHAR(2) NULL,
    discoverable BOOL NULL,
    follower_count INT NOT NULL,
    following_count INT NOT NULL,
    statuses_count INT NOT NULL,
    owned_by TEXT NULL,
    private_key TEXT NULL,
    local BOOLEAN NOT NULL,
    inbox_url TEXT NULL,
    outbox_url TEXT NULL,
    shared_inbox_url TEXT NULL,
    url TEXT NULL
);

CREATE INDEX accounts_username ON accounts (username);
CREATE INDEX accounts_owned_by ON accounts (owned_by);

CREATE TABLE account_fields (
    id UUID PRIMARY KEY NOT NULL,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    sort_order INT NOT NULL
);

CREATE INDEX account_fields_account_id ON account_fields (account_id);