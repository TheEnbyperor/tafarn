ALTER TABLE accounts
    DROP COLUMN avatar_file,
    DROP COLUMN avatar_content_type,
    DROP COLUMN avatar_remote_url,
    DROP COLUMN header_file,
    DROP COLUMN header_content_type,
    DROP COLUMN header_remote_url,
    ADD COLUMN avatar TEXT NULL,
    ADD COLUMN header TEXT NULL;