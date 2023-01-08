CREATE TABLE media (
    id UUID PRIMARY KEY NOT NULL,
    iid SERIAL,
    media_type VARCHAR(255) NOT NULL,
    file TEXT NULL,
    content_type VARCHAR(255) NULL,
    remote_url TEXT NULL,
    preview_file TEXT NULL,
    preview_content_type VARCHAR(255) NULL,
    blurhash TEXT NULL,
    focus_x DOUBLE PRECISION NULL,
    focus_y DOUBLE PRECISION NULL,
    original_width INT NULL,
    original_height INT NULL,
    preview_width INT NULL,
    preview_height INT NULL,
    created_at TIMESTAMP NOT NULL,
    description TEXT NULL,
    owned_by TEXT NULL
);

CREATE INDEX media_iid_idx ON media (iid);