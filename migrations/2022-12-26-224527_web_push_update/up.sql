ALTER TABLE web_push_subscriptions
    ADD COLUMN "status" BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN "follow_request" BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN "update" BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN "admin_sign_up" BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN "admin_report" BOOLEAN NOT NULL DEFAULT FALSE;