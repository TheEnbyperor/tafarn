CREATE TABLE notifications (
    id UUID NOT NULL PRIMARY KEY,
    notification_type VARCHAR NOT NULL,
    account UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    cause UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status UUID NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
)