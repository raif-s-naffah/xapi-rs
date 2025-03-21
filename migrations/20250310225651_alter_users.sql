-- Add migration script here

-- See https://github.com/raif-s-naffah/xapi-rs/issues/18 for details.
-- Default Role is Guest (0).
ALTER TABLE users
  DROP COLUMN admin,
  ADD COLUMN role SMALLINT NOT NULL DEFAULT 0;

-- Set test user's role to be Root (4)
UPDATE users SET role = 4 WHERE id = 1;
