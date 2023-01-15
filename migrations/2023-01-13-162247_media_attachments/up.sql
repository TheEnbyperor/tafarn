CREATE TABLE media_attachments (
    status UUID NOT NULL REFERENCES statuses(id) ON DELETE CASCADE,
    media UUID NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    PRIMARY KEY (status, media)
);

ALTER TABLE statuses
    ADD COLUMN text_source TEXT NULL,
    ADD COLUMN spoiler_text_source TEXT NULL;