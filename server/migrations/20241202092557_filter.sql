-- Add migration script here

-- We use this table to allocate a unique integer which we then prefix w/ the
-- letter 'v' to form the stem of one or more database Views when processing
-- GET /statements requests that do not target a single Statement.
-- We also store a timestamp of when this allocation occurs b/c after a certain
-- Time-To-Live elapses, those Views and the associated row in this table are
-- removed.
--
-- See https://github.com/raif-s-naffah/xapi-rs/issues/1
--
CREATE TABLE IF NOT EXISTS filter (
	id bigserial NOT NULL PRIMARY KEY,
	created timestamp(0) NOT NULL DEFAULT LOCALTIMESTAMP(0)
);
