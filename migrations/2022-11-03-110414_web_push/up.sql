CREATE TABLE web_push_subscriptions (
    id UUID PRIMARY KEY NOT NULL,
    token_id UUID REFERENCES oauth_token(id) ON DELETE CASCADE UNIQUE,
    account_id UUID REFERENCES accounts(id) ON DELETE CASCADE,
    endpoint TEXT NOT NULL,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    follow BOOLEAN NOT NULL DEFAULT FALSE,
    favourite BOOLEAN NOT NULL DEFAULT FALSE,
    reblog BOOLEAN NOT NULL DEFAULT FALSE,
    mention BOOLEAN NOT NULL DEFAULT FALSE,
    poll BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX web_push_subscriptions_token_id ON web_push_subscriptions (token_id);
CREATE INDEX web_push_subscriptions_account_id ON web_push_subscriptions (account_id);