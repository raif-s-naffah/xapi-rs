-- Add migration script here

-- See https://github.com/raif-s-naffah/xapi-rs/issues/5 for details.
-- IMPORTANT (rsn) 20250110 - 'user' is a PostgreSQL reserved keyword :( hence
-- the plural form for the table's name.
CREATE TABLE IF NOT EXISTS users (
    id SERIAL NOT NULL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    credentials BIGINT NOT NULL UNIQUE,
    admin BOOLEAN DEFAULT FALSE,
    manager_id INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN DEFAULT TRUE,
    created TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated TIMESTAMPTZ NULL DEFAULT NOW()
);

CREATE OR REPLACE TRIGGER users_updated
  BEFORE INSERT OR UPDATE ON users
  FOR EACH ROW EXECUTE FUNCTION update_timestamp();

-- Ideally this INSERT should be in a migrations file of its own conditionally
-- applied if/when LaRS is not running in Legacy mode. However it turns out 
-- conditionally applying migrations w/ SQLx is not a straightforward matter.
-- see https://github.com/launchbadge/sqlx/discussions/3676.
-- For this reason, we always add the "test" user whose 'email' is hard-wired
-- and has credentials computed from a blank password (see TEST_USER_PLAIN_TOKEN).
INSERT INTO users (email, credentials) VALUES ('test@my.xapi.net', 2175704399);
