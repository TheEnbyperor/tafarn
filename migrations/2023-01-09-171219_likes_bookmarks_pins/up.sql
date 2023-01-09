CREATE TABLE likes (
    id UUID PRIMARY KEY NOT NULL,
    iid SERIAL,
    status UUID NULL REFERENCES statuses(id),
    account UUID NOT NULL REFERENCES accounts(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    url TEXT NULL,
    local BOOLEAN NOT NULL DEFAULT FALSE,
    status_url TEXT NULL
);

CREATE TABLE bookmarks (
    id UUID PRIMARY KEY NOT NULL,
    iid SERIAL,
    status UUID NOT NULL REFERENCES statuses(id),
    account UUID NOT NULL REFERENCES accounts(id)
);

CREATE TABLE pins (
    id UUID PRIMARY KEY NOT NULL,
    iid SERIAL,
    status UUID NOT NULL REFERENCES statuses(id),
    account UUID NOT NULL REFERENCES accounts(id)
);