CREATE TABLE public_keys (
    id UUID PRIMARY KEY NOT NULL,
    key_id TEXT NOT NULL,
    user_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key TEXT NOT NULL
);

CREATE INDEX public_keys_key_id ON public_keys(key_id);