DROP TABLE media_attachments;

ALTER TABLE statuses
    DROP COLUMN text_source,
    DROP COLUMN spoiler_text_sounce;