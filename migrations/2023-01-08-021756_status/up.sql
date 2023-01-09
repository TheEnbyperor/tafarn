CREATE TABLE statuses (
    id UUID PRIMARY KEY NOT NULL,
    iid SERIAL,
    url TEXT NOT NULL,
    uri TEXT NULL,
    text TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    in_reply_to_id UUID NULL REFERENCES statuses(id) ON DELETE SET NULL,
    boost_of_id UUID NULL REFERENCES statuses(id) ON DELETE CASCADE,
    in_reply_to_url TEXT NULL,
    boost_of_url TEXT NULL,
    sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    spoiler_text TEXT NOT NULL DEFAULT '',
    language TEXT NULL,
    local BOOLEAN NOT NULL DEFAULT TRUE,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    deleted_at TIMESTAMP NULL,
    edited_at TIMESTAMP NULL,
    public BOOLEAN NOT NULL DEFAULT FALSE,
    visible BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX statuses_iid_idx ON statuses (iid);

CREATE TABLE status_media_attachments (
    status_id UUID NOT NULL REFERENCES statuses(id) ON DELETE CASCADE,
    media_attachment_id UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    attachment_order INTEGER NOT NULL,
    PRIMARY KEY (status_id, media_attachment_id)
);

ALTER TABLE accounts ADD COLUMN follower_collection_url TEXT NULL;

CREATE TABLE status_audiences (
    id UUID PRIMARY KEY NOT NULL,
    status_id UUID NOT NULL REFERENCES statuses(id) ON DELETE CASCADE,
    mention BOOLEAN NOT NULL DEFAULT FALSE,
    account UUID NULL REFERENCES accounts(id) ON DELETE CASCADE,
    account_followers UUID NULL REFERENCES accounts(id) ON DELETE CASCADE
);