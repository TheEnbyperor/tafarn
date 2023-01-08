CREATE TABLE home_timeline (
    id SERIAL PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    status_id UUID NOT NULL REFERENCES statuses(id) ON DELETE CASCADE
);

CREATE TABLE public_timeline (
     id SERIAL PRIMARY KEY,
     status_id UUID NOT NULL REFERENCES statuses(id) ON DELETE CASCADE
);