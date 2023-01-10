CREATE TABLE account_notes (
    account UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    owner UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    note TEXT NOT NULL,
    PRIMARY KEY (account, owner)
);