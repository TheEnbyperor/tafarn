ALTER TABLE accounts
    ADD COLUMN avatar_file TEXT NULL,
    ADD COLUMN avatar_content_type VARCHAR(255) NULL,
    ADD COLUMN avatar_remote_url TEXT NULL,
    ADD COLUMN header_file TEXT NULL,
    ADD COLUMN header_content_type VARCHAR(255) NULL,
    ADD COLUMN header_remote_url TEXT NULL,
    DROP COLUMN avatar,
    DROP COLUMN header;