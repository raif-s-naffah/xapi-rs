-- Add migration script here

-- see https://github.com/raif-s-naffah/xapi-rs/issues/34 for details
-- add new columns...
ALTER TABLE users
  ADD COLUMN salt UUID DEFAULT (uuidv7()),
  ADD COLUMN credentials2 BIGINT DEFAULT 0,
  ADD COLUMN ready BOOLEAN NOT NULL DEFAULT FALSE;
