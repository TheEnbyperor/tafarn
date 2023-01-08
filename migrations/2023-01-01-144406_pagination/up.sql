ALTER TABLE accounts ADD COLUMN iid SERIAL;
ALTER TABLE following ADD COLUMN iid SERIAL;
ALTER TABLE notifications ADD COLUMN iid SERIAL;

CREATE INDEX accounts_iid_idx ON accounts (iid);
CREATE INDEX following_iid_idx ON following (iid);
CREATE INDEX notifications_iid_idx ON notifications (iid);